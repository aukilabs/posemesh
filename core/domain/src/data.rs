use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DomainData {
    pub domain_id: String,         // Rust-friendly string
    pub hash: String,              // Merkle root as string
    pub name: String,              // Name (max length 256)
    pub data_type: String,             // Type (max length 128)
    pub properties: HashMap<String, String>,  // Metadata in JSON format
    pub content: Vec<u8>,          // Content as binary
    pub content_size: usize,       // Size of the content
}

impl DomainData {
    pub fn new(
        domain_id: String,
        name: String,
        data_type: String,
        properties: HashMap<String, String>,
    ) -> Self {
        DomainData {
            domain_id,
            hash: String::new(),
            name,
            data_type,
            properties,
            content: Vec::new(),
            content_size: 0,
        }
    }
}
