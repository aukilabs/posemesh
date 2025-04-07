use std::{collections::{HashMap, VecDeque}, fmt::write, sync::Arc};
use futures::{channel::mpsc::{self, channel}, SinkExt, StreamExt};
use tokio::{spawn, sync::{oneshot, Mutex}};
use tokio_postgres::{types::ToSql, Client, NoTls};
use uuid::Uuid;
use crate::protobuf::domain_data::{Data, Metadata, Query};
use super::{common::{data_id_generator, DataReader, DataWriter, Datastore, DomainData, DomainError, Reader, ReliableDataProducer, Writer}, fs::from_path_to_hash};
use async_trait::async_trait;

pub(crate) struct InstantPush {
    pub response: oneshot::Sender<Result<Metadata, DomainError>>,
    pub data: Metadata,
}

pub struct MetadataProducer {
    pub(crate) writer: mpsc::Sender<InstantPush>,
}

pub struct MetadataDomainData {
    pub hash: String,
}

#[async_trait]
impl DomainData for MetadataDomainData {
    async fn push_chunk(&mut self, _: &[u8], _: bool) -> Result<String, DomainError> {
        Ok(self.hash.clone())
    }
}

#[async_trait]
impl ReliableDataProducer for MetadataProducer {
    async fn push(&mut self, data: &Metadata) -> Result<Box<dyn DomainData>, DomainError> {
        let (response, receiver) = oneshot::channel();
        let push = InstantPush {
            response,
            data: data.clone(),
        };
        self.writer.send(push).await.unwrap();
        match receiver.await.unwrap() {
            Ok(metadata) => Ok(Box::new(MetadataDomainData { hash: metadata.hash.unwrap() })),
            Err(e) => Err(e),
        }
    }

    async fn is_completed(&self) -> bool {
        true
    }

    async fn close(&mut self) {
        let _ = self.writer.close().await;
    }
}

#[derive(Clone)]
pub struct MetadataStore {
    client: Arc<Mutex<Client>>,
}

impl MetadataStore {
    pub async fn new(conn_str: &str) -> Result<MetadataStore, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // Spawn a task to manage the connection
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(MetadataStore { client: Arc::new(Mutex::new(client)) })
    }
}

impl Query {
    pub fn to_sql(&self, domain_id: Uuid) -> (String, Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>) {
        let mut sql = String::from("SELECT id, name, data_type, data_size, link, domain_id FROM domain_data WHERE domain_id = $1");
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        params.push(Box::new(domain_id));
        let mut param_index = 2;

        if !self.ids.is_empty() {
            sql.push_str(&format!(" AND id = ANY({})", param_index));
            params.push(Box::new(self.ids.clone().iter().map(|id| Uuid::parse_str(id).unwrap()).collect::<Vec<Uuid>>()));
            param_index += 1;
        }

        if let Some(name_regexp) = &self.name_regexp {
            sql.push_str(&format!(" AND name ~ ${}", param_index));
            params.push(Box::new(name_regexp.clone()));
            param_index += 1;
        }

        if let Some(data_type_regexp) = &self.data_type_regexp {
            sql.push_str(&format!(" AND data_type ~ ${}", param_index));
            params.push(Box::new(data_type_regexp.clone()));
            param_index += 1;
        }

        if !self.names.is_empty() {
            sql.push_str(&format!(" AND name = ANY({})", param_index));
            params.push(Box::new(self.names.clone()));
            param_index += 1;
        }

        if !self.data_types.is_empty() {
            sql.push_str(&format!(" AND data_type = ANY({})", param_index));
            params.push(Box::new(self.data_types.clone()));
        }

        (sql, params)
    }
}

#[async_trait]
impl Datastore for MetadataStore {
    async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> DataReader {
        let domain_id_res = Uuid::parse_str(&domain_id);
        if let Err(e) = domain_id_res {
            panic!("{}", e);
        }
        let domain_id = domain_id_res.unwrap();
        let (sql, params) = query.to_sql(domain_id);
        let (mut writer, reader) = channel::<Result<Data, DomainError>>(240);
        let client = self.client.clone();

        spawn(async move {
            let client = client.lock().await;
            let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|s| s.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, &params_refs[..]).await.unwrap();
            for row in rows {
                let link: String = row.get("link");
                let hash = from_path_to_hash(&link).unwrap();
                let domain_id: Uuid = row.get("domain_id");
                let id: Uuid = row.get("id");
                let size: i64 = row.get("data_size");
                let data = Data {
                    domain_id: domain_id.to_string(),
                    metadata: Metadata {
                        id: Some(id.to_string()),
                        name: row.get("name"),
                        data_type: row.get("data_type"),
                        properties: HashMap::new(),
                        size: size as u32,
                        link: Some(link.clone()),
                        hash: Some(hash.to_string()),
                    },
                    content: vec![],
                };
                writer.send(Ok(data)).await.unwrap();
            }

            if !keep_alive {
                let _ = writer.close().await;
            }
        });

        reader
    }

    async fn upsert(&mut self, domain_id: String) -> Box<dyn ReliableDataProducer> {
        let client = self.client.clone();
        
        let (writer, mut reader) = channel::<InstantPush>(240);
        
        spawn(async move {
            while let Some(push) = reader.next().await {
                let mut metadata = push.data;
                let response_writer = push.response;
                let mut sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (name, domain_id) DO UPDATE SET data_type=$3,data_size=$4,link=$5,updated_at=now()";
                let id = metadata.id.clone().unwrap_or(data_id_generator());
                if metadata.id.is_none() {
                    metadata.id = Some(id.clone());
                } else {
                    sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO UPDATE SET name=$2,data_type=$3,data_size=$4,link=$5,domain_id=$6,updated_at=now()";
                }
                let id_parse_result = Uuid::parse_str(&id.clone());
                if let Err(e) = id_parse_result {
                    tracing::error!("{}", e);
                    response_writer.send(Err(DomainError::Cancelled(format!("Invalid id: {} {}", id, e)))).expect("send error");
                    continue;
                }
                let id = id_parse_result.unwrap();
                let size = i64::from(metadata.size);
                
                let domain_id_parse_result = Uuid::parse_str(&domain_id.clone());
                if let Err(e) = domain_id_parse_result {
                    tracing::error!("{}", e);
                    response_writer.send(Err(DomainError::Cancelled(format!("Invalid domain id: {} {}", domain_id, e)))).expect("send error");
                    continue;
                }
                let domain_id = domain_id_parse_result.unwrap();

                let client = client.lock().await;
                if let Err(e) = client.execute(sql, &[
                    &id,
                    &metadata.name,
                    &metadata.data_type,
                    &size,
                    &metadata.link,
                    &domain_id,
                ]).await {
                    tracing::error!("{}", e);
                    response_writer.send(Err(DomainError::Interrupted)).expect("send error");
                    continue;
                }
                response_writer.send(Ok(metadata)).expect("send error");
            }
        });

        Box::new(MetadataProducer { writer })
    }
}
