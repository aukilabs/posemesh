// Automatically generated rust module for 'domain_data.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::collections::HashMap;
type KVMap<K, V> = HashMap<K, V>;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Metadata {
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub id: String,
    pub hash: Option<String>,
    pub properties: KVMap<String, String>,
}

impl<'a> MessageRead<'a> for Metadata {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.data_type = r.read_string(bytes)?.to_owned(),
                Ok(24) => msg.size = r.read_uint32(bytes)?,
                Ok(34) => msg.id = r.read_string(bytes)?.to_owned(),
                Ok(42) => msg.hash = Some(r.read_string(bytes)?.to_owned()),
                Ok(50) => {
                    let (key, value) = r.read_map(bytes, |r, bytes| Ok(r.read_string(bytes)?.to_owned()), |r, bytes| Ok(r.read_string(bytes)?.to_owned()))?;
                    msg.properties.insert(key, value);
                }
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Metadata {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + 1 + sizeof_len((&self.data_type).len())
        + 1 + sizeof_varint(*(&self.size) as u64)
        + 1 + sizeof_len((&self.id).len())
        + self.hash.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.properties.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).len()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.name))?;
        w.write_with_tag(18, |w| w.write_string(&**&self.data_type))?;
        w.write_with_tag(24, |w| w.write_uint32(*&self.size))?;
        w.write_with_tag(34, |w| w.write_string(&**&self.id))?;
        if let Some(ref s) = self.hash { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        for (k, v) in self.properties.iter() { w.write_with_tag(50, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).len()), 10, |w| w.write_string(&**k), 18, |w| w.write_string(&**v)))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Query {
    pub ids: Vec<String>,
    pub name_regexp: Option<String>,
    pub data_type_regexp: Option<String>,
    pub names: Vec<String>,
    pub data_types: Vec<String>,
    pub metadata_only: bool,
}

impl<'a> MessageRead<'a> for Query {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.ids.push(r.read_string(bytes)?.to_owned()),
                Ok(18) => msg.name_regexp = Some(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.data_type_regexp = Some(r.read_string(bytes)?.to_owned()),
                Ok(34) => msg.names.push(r.read_string(bytes)?.to_owned()),
                Ok(42) => msg.data_types.push(r.read_string(bytes)?.to_owned()),
                Ok(48) => msg.metadata_only = r.read_bool(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Query {
    fn get_size(&self) -> usize {
        0
        + self.ids.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.name_regexp.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.data_type_regexp.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.names.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.data_types.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + 1 + sizeof_varint(*(&self.metadata_only) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.ids { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.name_regexp { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.data_type_regexp { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        for s in &self.names { w.write_with_tag(34, |w| w.write_string(&**s))?; }
        for s in &self.data_types { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        w.write_with_tag(48, |w| w.write_bool(*&self.metadata_only))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Data {
    pub domain_id: String,
    pub metadata: domain_data::Metadata,
    pub content: Vec<u8>,
}

impl<'a> MessageRead<'a> for Data {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.domain_id = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.metadata = r.read_message::<domain_data::Metadata>(bytes)?,
                Ok(26) => msg.content = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Data {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.domain_id).len())
        + 1 + sizeof_len((&self.metadata).get_size())
        + 1 + sizeof_len((&self.content).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.domain_id))?;
        w.write_with_tag(18, |w| w.write_message(&self.metadata))?;
        w.write_with_tag(26, |w| w.write_bytes(&**&self.content))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct UpsertMetadata {
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub is_new: bool,
    pub id: String,
    pub properties: KVMap<String, String>,
}

impl<'a> MessageRead<'a> for UpsertMetadata {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.data_type = r.read_string(bytes)?.to_owned(),
                Ok(24) => msg.size = r.read_uint32(bytes)?,
                Ok(32) => msg.is_new = r.read_bool(bytes)?,
                Ok(42) => msg.id = r.read_string(bytes)?.to_owned(),
                Ok(50) => {
                    let (key, value) = r.read_map(bytes, |r, bytes| Ok(r.read_string(bytes)?.to_owned()), |r, bytes| Ok(r.read_string(bytes)?.to_owned()))?;
                    msg.properties.insert(key, value);
                }
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for UpsertMetadata {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + 1 + sizeof_len((&self.data_type).len())
        + 1 + sizeof_varint(*(&self.size) as u64)
        + 1 + sizeof_varint(*(&self.is_new) as u64)
        + 1 + sizeof_len((&self.id).len())
        + self.properties.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).len()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.name))?;
        w.write_with_tag(18, |w| w.write_string(&**&self.data_type))?;
        w.write_with_tag(24, |w| w.write_uint32(*&self.size))?;
        w.write_with_tag(32, |w| w.write_bool(*&self.is_new))?;
        w.write_with_tag(42, |w| w.write_string(&**&self.id))?;
        for (k, v) in self.properties.iter() { w.write_with_tag(50, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).len()), 10, |w| w.write_string(&**k), 18, |w| w.write_string(&**v)))?; }
        Ok(())
    }
}

