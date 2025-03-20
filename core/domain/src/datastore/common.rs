
use std::{collections::HashSet, error::Error, sync::Arc};

use crate::protobuf::domain_data::{self, Data};
use async_trait::async_trait;
use futures::{channel::mpsc::{self, Receiver, Sender}, lock::Mutex, SinkExt, StreamExt};
use uuid::Uuid;

pub type Reader<T> = Receiver<Result<T, DomainError>>;
pub type Writer<T> = Sender<Result<T, DomainError>>;
pub type DataWriter = Writer<Data>;
pub type DataReader = Reader<Data>;

#[cfg(not(target_family = "wasm"))]
use tokio::task::spawn;

#[cfg(target_family = "wasm")]
use wasm_bindgen_futures::spawn_local as spawn;

// Define a custom error type
#[derive(Debug)]
pub enum DomainError {
    NotFound,
    Interrupted,
    Cancelled,
}

impl Error for DomainError {}
impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DomainError::NotFound => write!(f, "Not found"),
            DomainError::Interrupted => write!(f, "Interrupted"),
            DomainError::Cancelled => write!(f, "Cancelled"),
        }
    }
}


#[async_trait]
pub trait Datastore: Send + Sync {
    async fn consume(self: &mut Self, domain_id: String, query: domain_data::Query, keep_alive: bool) -> DataReader;
    async fn produce(self: &mut Self, domain_id: String) -> ReliableDataProducer;
}

pub fn data_id_generator() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Clone)]
pub struct ReliableDataProducer {
    writer: DataWriter,
    pendings: Arc<Mutex<HashSet<String>>>,
    pub progress: Arc<Mutex<Receiver<i32>>>,
    total: Arc<Mutex<i32>>,
}

impl ReliableDataProducer {
    pub fn new(mut response: Reader<domain_data::Metadata>, writer: DataWriter) -> Self {
        let pendings = Arc::new(Mutex::new(HashSet::new()));
        let pending_clone = pendings.clone();
        let total: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
        let total_clone = total.clone();
        let (mut progress_sender, progress_receiver) = mpsc::channel(100);

        spawn(async move {
            while let Some(m) = response.next().await {
                match m {
                    Ok(metadata) => {
                        let id = metadata.id.unwrap_or("why no id".to_string());
                        let mut pendings = pending_clone.lock().await;
                        let total = total_clone.lock().await;
                        let completed = *total as usize - pendings.len() + 1;
                        pendings.remove(&id);
                        let progress = completed * 100 / *total as usize;
                        tracing::info!("back");
                        progress_sender.send(progress as i32).await.expect("can't send progress");
                        tracing::info!("progress: {}", progress);
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
        });

        Self {
            writer, progress: Arc::new(Mutex::new(progress_receiver)), pendings, total
        }
    }

    pub async fn push(&mut self, data: &domain_data::Data) -> Result<String, DomainError> {
        let res = self.writer.send(Ok(data.clone())).await;
        match res {
            Ok(_) => {
                let id = data.metadata.id.clone().unwrap_or("no id?".to_string());
                let mut pendings = self.pendings.lock().await;
                pendings.insert(id.clone());
                let mut total = self.total.lock().await;
                *total += 1;
                Ok(id)
            },
            Err(e) => {
                eprintln!("{}", e);
                Err(DomainError::Interrupted)
            },
        }
    }

    pub async fn is_completed(&self) -> bool {
        let pendings = self.pendings.lock();
        pendings.await.is_empty()
    }

    pub async fn close(mut self) {
        self.writer.close().await.expect("can't close writer");
        self.progress.lock().await.close();
        self.pendings.lock().await.clear();
    }
}
