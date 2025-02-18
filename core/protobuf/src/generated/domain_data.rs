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
pub struct DomainDataMetadata {
    pub name: String,
    pub data_type: String,
    pub size: u32,
    pub hash: String,
    pub properties: KVMap<String, String>,
}

impl<'a> MessageRead<'a> for DomainDataMetadata {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.data_type = r.read_string(bytes)?.to_owned(),
                Ok(24) => msg.size = r.read_uint32(bytes)?,
                Ok(34) => msg.hash = r.read_string(bytes)?.to_owned(),
                Ok(42) => {
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

impl MessageWrite for DomainDataMetadata {
    fn get_size(&self) -> usize {
        0
        + if self.name == String::default() { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.data_type == String::default() { 0 } else { 1 + sizeof_len((&self.data_type).len()) }
        + if self.size == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.size) as u64) }
        + if self.hash == String::default() { 0 } else { 1 + sizeof_len((&self.hash).len()) }
        + self.properties.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).len()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.name != String::default() { w.write_with_tag(10, |w| w.write_string(&**&self.name))?; }
        if self.data_type != String::default() { w.write_with_tag(18, |w| w.write_string(&**&self.data_type))?; }
        if self.size != 0u32 { w.write_with_tag(24, |w| w.write_uint32(*&self.size))?; }
        if self.hash != String::default() { w.write_with_tag(34, |w| w.write_string(&**&self.hash))?; }
        for (k, v) in self.properties.iter() { w.write_with_tag(42, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).len()), 10, |w| w.write_string(&**k), 18, |w| w.write_string(&**v)))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DataQuery {
    pub domain_id: String,
    pub hashes: Vec<String>,
    pub name: String,
    pub data_type: String,
}

impl<'a> MessageRead<'a> for DataQuery {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.domain_id = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.hashes.push(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(34) => msg.data_type = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for DataQuery {
    fn get_size(&self) -> usize {
        0
        + if self.domain_id == String::default() { 0 } else { 1 + sizeof_len((&self.domain_id).len()) }
        + self.hashes.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + if self.name == String::default() { 0 } else { 1 + sizeof_len((&self.name).len()) }
        + if self.data_type == String::default() { 0 } else { 1 + sizeof_len((&self.data_type).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.domain_id != String::default() { w.write_with_tag(10, |w| w.write_string(&**&self.domain_id))?; }
        for s in &self.hashes { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if self.name != String::default() { w.write_with_tag(26, |w| w.write_string(&**&self.name))?; }
        if self.data_type != String::default() { w.write_with_tag(34, |w| w.write_string(&**&self.data_type))?; }
        Ok(())
    }
}

