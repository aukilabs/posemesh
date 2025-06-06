use std::{collections::HashMap, sync::Arc};
use async_trait::async_trait;
use futures::{channel::mpsc::{channel, Sender}, SinkExt};
#[cfg(test)]
use mockall::automock;
use tokio::{spawn, sync::Mutex};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;
use crate::protobuf::domain_data::Query;
use super::{common::{data_id_generator, DomainError, Reader}, fs::from_path_to_hash};

pub struct UpsertMetadata {
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub id: String,
    pub is_new: bool,
    pub link: String,
    pub hash: String,
    pub properties: HashMap<String, String>,
}

#[derive(Clone)]
pub struct Metadata {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub properties: HashMap<String, String>,
    pub link: String,
    pub hash: String,
}

#[derive(Clone)]
pub struct PGMetadataStore {
    pg_client: Arc<Mutex<Client>>,
    listeners: Arc<Mutex<HashMap<String, HashMap<String, Listener>>>>,
}

struct Listener {
    query: Query,
    sender: Sender<Result<Metadata, DomainError>>,
}

impl Listener {
    pub fn new(query: Query, sender: Sender<Result<Metadata, DomainError>>) -> Self {
        Self { query, sender }
    }
}

pub struct MetadataReader {
    pub(crate) reader: Reader<Metadata>,
    pub(crate) id: Option<String>,
}

impl PGMetadataStore {
    pub async fn new(conn_str: &str) -> Result<PGMetadataStore, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // Spawn a task to manage the connection
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Database connection error: {}", e);
            }
        });

        Ok(PGMetadataStore { pg_client: Arc::new(Mutex::new(client)), listeners: Arc::new(Mutex::new(HashMap::new())) })
    }
}

impl Query {
    pub fn to_where_clause(&self, domain_id: Uuid) ->  (String, Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>) {
        let mut where_clause = String::from("domain_id = $1");
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        params.push(Box::new(domain_id));
        let mut param_index = 2;

        if !self.ids.is_empty() {
            where_clause.push_str(&format!(" AND id = ANY(${})", param_index));
            params.push(Box::new(self.ids.clone().iter().map(|id| Uuid::parse_str(id).unwrap()).collect::<Vec<Uuid>>()));
            param_index += 1;
        }

        if let Some(name_regexp) = &self.name_regexp {
            where_clause.push_str(&format!(" AND name ~ ${}", param_index));
            params.push(Box::new(name_regexp.clone()));
            param_index += 1;
        }

        if let Some(data_type_regexp) = &self.data_type_regexp {
            where_clause.push_str(&format!(" AND data_type ~ ${}", param_index));
            params.push(Box::new(data_type_regexp.clone()));
            param_index += 1;
        }

        if !self.names.is_empty() {
            where_clause.push_str(&format!(" AND name = ANY(${})", param_index));
            params.push(Box::new(self.names.clone()));
            param_index += 1;
        }

        if !self.data_types.is_empty() {
            where_clause.push_str(&format!(" AND data_type = ANY(${})", param_index));
            params.push(Box::new(self.data_types.clone()));
        }

        (where_clause, params)
    }
    pub fn to_select(&self, domain_id: Uuid) -> (String, Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>) {
        let mut sql = String::from("SELECT id, name, data_type, data_size, link, domain_id FROM domain_data WHERE ");
        let (where_clause, params) = self.to_where_clause(domain_id);
        sql.push_str(&where_clause);
        (sql, params)
    }

    pub fn to_exists(&self, domain_id: Uuid) -> (String, Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>) {
        let mut sql = String::from("SELECT EXISTS (SELECT 1 FROM domain_data WHERE ");
        let (where_clause, params) = self.to_where_clause(domain_id);
        sql.push_str(&where_clause);
        sql.push_str(")");
        (sql, params)
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait MetadataStore: Send + Sync {
    async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> Result<MetadataReader, DomainError>;
    async fn upsert(&mut self, domain_id: String, metadata: UpsertMetadata) -> Result<(), DomainError>;
    async fn close_reader(&mut self, domain_id: String, reader: MetadataReader);
}

#[async_trait]
impl MetadataStore for PGMetadataStore {
    async fn close_reader(&mut self, domain_id: String, mut reader: MetadataReader) {
        if let Some(id) = reader.id {
            let mut listeners = self.listeners.lock().await;
            tracing::info!("Removing listener {} for domain_id: {}", id, domain_id);
            listeners.entry(domain_id).and_modify(|listeners| {
                if let Some(mut listener) = listeners.remove(&id) {
                    let _ = listener.sender.close();
                }
            });
        }
        reader.reader.close();
    }
    async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> Result<MetadataReader, DomainError> {
        let domain_id = Uuid::parse_str(&domain_id).map_err(|e| DomainError::Invalid("domain_id".to_string(), domain_id, e.to_string()))?;
        let (sql, params) = query.to_select(domain_id);
        let (mut writer, reader) = channel::<Result<Metadata, DomainError>>(240);
        let client = self.pg_client.clone();
        let listeners = self.listeners.clone();
        let mut listener_id: Option<String> = None;
        if keep_alive {
            listener_id = Some(data_id_generator());
        }
        let listener_id_clone = listener_id.clone();
        spawn(async move {
            let client = client.lock().await;
            let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|s| s.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let query_result = client.query(&sql, &params_refs[..]).await;
            if query_result.is_err() {
                let _ = writer.send(Err(DomainError::PostgresError(query_result.err().unwrap()))).await;
                return;
            }
            drop(client);
            let rows = query_result.unwrap();
            for row in rows {
                let link: String = row.get("link");
                let hash = from_path_to_hash(&link).unwrap();
                let id: Uuid = row.get("id");
                let size: i64 = row.get("data_size");
                let data = Metadata {
                    id: id.to_string(),
                    name: row.get("name"),
                    data_type: row.get("data_type"),
                    properties: HashMap::new(),
                    size: size as u32,
                    hash: hash.to_string(),
                    link,
                };
                writer.send(Ok(data)).await.unwrap();
            }

            if !keep_alive {
                let _ = writer.close().await;
            } else {
                let listener = Listener::new(query, writer.clone());
                let mut listeners = listeners.lock().await;
                listeners.entry(domain_id.to_string()).or_insert(HashMap::new()).insert(listener_id_clone.unwrap(), listener);
            }
        });

        Ok(MetadataReader { reader, id: listener_id })
    }

    async fn upsert(&mut self, domain_id: String, metadata: UpsertMetadata) -> Result<(), DomainError> {
        let client = self.pg_client.clone();
        
        let mut sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (name, domain_id) DO UPDATE SET data_type=$3,data_size=$4,link=$5,updated_at=now()";
        if !metadata.is_new {
            sql = "INSERT INTO domain_data (id, name, data_type, data_size, link, domain_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO UPDATE SET name=$2,data_type=$3,data_size=$4,link=$5,domain_id=$6,updated_at=now()";
        }
        let id = Uuid::parse_str(&metadata.id.clone()).map_err(|e| DomainError::Invalid("id".to_string(), metadata.id.clone(), e.to_string()))?;
        let size = i64::from(metadata.size);
        let domain_id = Uuid::parse_str(&domain_id.clone()).map_err(|e| DomainError::Invalid("domain_id".to_string(), domain_id, e.to_string()))?;

        let client = client.lock().await;
        client.execute(sql, &[
            &id,
            &metadata.name,
            &metadata.data_type,
            &size,
            &metadata.link,
            &domain_id,
        ]).await.map_err(|e| DomainError::PostgresError(e))?;
        drop(client);

        let listeners = self.listeners.clone();
        let updated_metadata = Metadata {
            id: metadata.id.clone(),
            name: metadata.name.clone(),
            data_type: metadata.data_type.clone(),
            size: metadata.size,
            properties: metadata.properties.clone(),
            link: metadata.link.clone(),
            hash: metadata.hash.clone(),
        };

        let client = self.pg_client.clone();
        spawn(async move {
            let mut listeners = listeners.lock().await;
            if let Some(domain_listeners) = listeners.get_mut(&domain_id.to_string()) {
                let id = updated_metadata.id.clone();
                for (_, listener) in domain_listeners.iter_mut() {
                    let client = client.lock().await;
                    if listener.query.ids.is_empty() {
                        let query = Query {
                            ids: vec![id.clone()],
                            ..listener.query.clone()
                        };
                        let (sql, params) = query.to_exists(domain_id);
                        let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|s| s.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
                        let query_result = client.query_one(&sql, &params_refs[..]).await;
                        drop(client);
                        if query_result.is_err() {
                            let _ = listener.sender.send(Err(DomainError::PostgresError(query_result.err().unwrap()))).await;
                            return;
                        }
                        let exists = query_result.unwrap().get(0);
                        if exists {
                            let _ = listener.sender.send(Ok(updated_metadata.clone())).await;
                        }
                    }
                    if !listener.query.ids.is_empty() && listener.query.ids.contains(&id) {
                        let _ = listener.sender.send(Ok(updated_metadata.clone())).await;
                    }
                }
            }
        });

        Ok(())
    }
}
