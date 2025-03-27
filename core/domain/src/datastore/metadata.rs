use std::{collections::{HashMap, VecDeque}, fmt::write, sync::Arc};
use futures::{channel::mpsc::channel, SinkExt, StreamExt};
use tokio::{spawn, sync::{oneshot, Mutex}};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;
use crate::protobuf::domain_data::{Data, Metadata, Query};
use super::common::{DataReader, DataWriter, Datastore, DomainError, Reader, ReliableDataProducer, Writer};
use async_trait::async_trait;

pub(crate) struct InstantPush {
    pub response: oneshot::Sender<Result<Metadata, DomainError>>,
    pub data: Data,
}

pub struct LocalProducer {
    pub writer: Writer<InstantPush>,
}

#[async_trait]
impl ReliableDataProducer for LocalProducer {
    async fn push(&mut self, data: &Data) -> Result<String, DomainError> {
        let (response, receiver) = oneshot::channel();
        let push = InstantPush {
            response,
            data: data.clone(),
        };
        self.writer.send(Ok(push)).await.unwrap();
        match receiver.await.unwrap() {
            Ok(metadata) => Ok(metadata.link.unwrap()),
            Err(e) => Err(e),
        }
    }

    async fn is_completed(&self) -> bool {
        true
    }

    async fn close(self) {
        drop(self.writer);
    }
}

#[derive(Clone)]
pub struct MetadataStore {
    client: Arc<Mutex<Client>>,
    path: String,
}

impl MetadataStore {
    pub async fn new(conn_str: &str, path: &str) -> Result<MetadataStore, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // Spawn a task to manage the connection
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Database connection error: {}", e);
            }
        });

        Ok(MetadataStore { client: Arc::new(Mutex::new(client)), path: path.to_string() })
    }
}

impl Query {
    pub fn to_sql(&self, domain_id: String) -> (String, Vec<String>) {
        let mut sql = String::from("SELECT id, name, data_type, data_size, link, domain_id FROM domain_data WHERE domain_id = $1");
        let mut params: VecDeque<String> = VecDeque::new();
        params.push_back(domain_id);
        let mut param_index = 2;

        if !self.ids.is_empty() {
            let placeholders: Vec<String> = self.ids.iter().enumerate()
                .map(|(i, _)| format!("${}", param_index + i))
                .collect();
            sql.push_str(&format!(" AND id IN ({})", placeholders.join(", ")));
            params.extend(self.ids.clone());
            param_index += self.ids.len();
        }

        if let Some(name_regexp) = &self.name_regexp {
            sql.push_str(&format!(" AND name ~ ${}", param_index));
            params.push_back(name_regexp.clone());
            param_index += 1;
        }

        if let Some(data_type_regexp) = &self.data_type_regexp {
            sql.push_str(&format!(" AND data_type ~ ${}", param_index));
            params.push_back(data_type_regexp.clone());
            param_index += 1;
        }

        if !self.names.is_empty() {
            let placeholders: Vec<String> = self.names.iter().enumerate()
                .map(|(i, _)| format!("${}", param_index + i))
                .collect();
            sql.push_str(&format!(" AND name IN ({})", placeholders.join(", ")));
            params.extend(self.names.clone());
            param_index += self.names.len();
        }

        if !self.data_types.is_empty() {
            let placeholders: Vec<String> = self.data_types.iter().enumerate()
                .map(|(i, _)| format!("${}", param_index + i))
                .collect();
            sql.push_str(&format!(" AND data_type IN ({})", placeholders.join(", ")));
            params.extend(self.data_types.clone());
        }

        (sql, params.into_iter().collect())
    }
}

#[async_trait]
impl Datastore for MetadataStore {
    async fn consume(&mut self, domain_id: String, query: Query, keep_alive: bool) -> DataReader {
        let (sql, params) = query.to_sql(domain_id);
        let (mut writer, reader) = channel::<Result<Data, DomainError>>(240);
        let client = self.client.clone();

        spawn(async move {
            let client = client.lock().await;
            let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let rows = client.query(&sql, &params_refs[..]).await.unwrap();
            for row in rows {
                let link: String = row.get("link");
                let hash = link.split('.').last().unwrap();
                let data = Data {
                    domain_id: row.get("domain_id"),
                    metadata: Metadata {
                        id: row.get("id"),
                        name: row.get("name"),
                        data_type: row.get("data_type"),
                        properties: HashMap::new(),
                        size: row.get("size"),
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

    async fn produce(&mut self, domain_id: String) -> Box<dyn ReliableDataProducer> {
        let client = self.client.clone();
        
        let (writer, mut reader) = channel::<Result<InstantPush, DomainError>>(240);
        
        let path = self.path.clone();
        let mut writer_clone = writer.clone();
        spawn(async move {
            let client = client.lock().await;
            while let Some(row) = reader.next().await {
                match row {
                    Ok(push) => {
                        let mut metadata = push.data.metadata;
                        let response_writer = push.response;
                        let mut sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (name, domain_id) DO UPDATE SET data_type=$3,size=$4,link=$5,updated_at=now()";
                        if metadata.id.is_none() {
                            let id = Uuid::new_v4().to_string();
                            metadata.id = Some(id.clone());
                            metadata.link = Some(format!("{}/{}/{}.{}", path, domain_id, id, metadata.hash.as_ref().unwrap()));
                        } else {
                            metadata.link = Some(format!("{}/{}/{}.{}", path, domain_id, metadata.id.as_ref().unwrap(), metadata.hash.as_ref().unwrap()));
                            sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO UPDATE SET name=$2,data_type=$3,size=$4,link=$5,domain_id=$6,updated_at=now()";
                        }
                        if let Err(e) = client.execute(sql, &[
                            &metadata.id,
                            &metadata.name,
                            &metadata.data_type,
                            &metadata.size,
                            &metadata.link,
                            &domain_id,
                        ]).await {
                            tracing::error!("{}", e);
                            response_writer.send(Err(DomainError::Interrupted)).expect("send error");
                            continue;
                        }
                        response_writer.send(Ok(metadata)).expect("send error");
                    }
                    Err(e) => {
                        let _ = writer_clone.send(Err(e)).await;
                        break;
                    }
                }
            }
        });

        Box::new(LocalProducer { writer })
    }
}
