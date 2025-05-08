use std::{collections::HashMap, sync::Arc};
use futures::{channel::mpsc::channel, SinkExt};
use tokio::{spawn, sync::Mutex};
use tokio_postgres::{Client, NoTls};
use uuid::Uuid;
use crate::protobuf::domain_data::Query;
use super::{common::{DomainError, Reader}, fs::from_path_to_hash};

pub(crate) struct UpsertMetadata {
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub id: String,
    pub is_new: bool,
    pub link: String,
    pub hash: String,
    pub properties: HashMap<String, String>,
}

pub(crate) struct Metadata {
    pub id: String,
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub properties: HashMap<String, String>,
    pub link: String,
    pub hash: String,
}

#[derive(Clone)]
pub struct MetadataStore {
    pg_client: Arc<Mutex<Client>>,
}

impl MetadataStore {
    pub async fn new(conn_str: &str) -> Result<MetadataStore, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(conn_str, NoTls).await?;

        // Spawn a task to manage the connection
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Database connection error: {}", e);
            }
        });

        Ok(MetadataStore { pg_client: Arc::new(Mutex::new(client)) })
    }
}

impl Query {
    pub fn to_sql(&self, domain_id: Uuid) -> (String, Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>>) {
        let mut sql = String::from("SELECT id, name, data_type, data_size, link, domain_id FROM domain_data WHERE domain_id = $1");
        let mut params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = Vec::new();
        params.push(Box::new(domain_id));
        let mut param_index = 2;

        if !self.ids.is_empty() {
            sql.push_str(&format!(" AND id = ANY(${})", param_index));
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
            sql.push_str(&format!(" AND name = ANY(${})", param_index));
            params.push(Box::new(self.names.clone()));
            param_index += 1;
        }

        if !self.data_types.is_empty() {
            sql.push_str(&format!(" AND data_type = ANY(${})", param_index));
            params.push(Box::new(self.data_types.clone()));
        }

        (sql, params)
    }
}

impl MetadataStore {
    pub(crate) async fn load(&mut self, domain_id: String, query: Query, keep_alive: bool) -> Result<Reader<Metadata>, DomainError> {
        let domain_id = Uuid::parse_str(&domain_id).map_err(|e| DomainError::Invalid("domain_id".to_string(), domain_id, e.to_string()))?;
        let (sql, params) = query.to_sql(domain_id);
        let (mut writer, reader) = channel::<Result<Metadata, DomainError>>(240);
        let client = self.pg_client.clone();

        spawn(async move {
            let client = client.lock().await;
            let params_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params.iter().map(|s| s.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync)).collect();
            let query_result = client.query(&sql, &params_refs[..]).await;
            if query_result.is_err() {
                let _ = writer.send(Err(DomainError::PostgresError(query_result.err().unwrap()))).await;
                return;
            }
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
            }
        });

        Ok(reader)
    }

    pub(crate) async fn upsert(&mut self, domain_id: String, metadata: UpsertMetadata) -> Result<(), DomainError> {
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
        Ok(())
    }
}
