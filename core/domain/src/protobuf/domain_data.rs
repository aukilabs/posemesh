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
                Ok(34) => msg.hash = Some(r.read_string(bytes)?.to_owned()),
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

impl MessageWrite for Metadata {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + 1 + sizeof_len((&self.data_type).len())
        + 1 + sizeof_varint(*(&self.size) as u64)
        + self.hash.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.properties.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_len((v).len()))).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.name))?;
        w.write_with_tag(18, |w| w.write_string(&**&self.data_type))?;
        w.write_with_tag(24, |w| w.write_uint32(*&self.size))?;
        if let Some(ref s) = self.hash { w.write_with_tag(34, |w| w.write_string(&**s))?; }
        for (k, v) in self.properties.iter() { w.write_with_tag(42, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_len((v).len()), 10, |w| w.write_string(&**k), 18, |w| w.write_string(&**v)))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Query {
    pub domain_id: String,
    pub hashes: Vec<String>,
    pub name: Option<String>,
    pub data_type: Option<String>,
}

impl<'a> MessageRead<'a> for Query {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.domain_id = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.hashes.push(r.read_string(bytes)?.to_owned()),
                Ok(26) => msg.name = Some(r.read_string(bytes)?.to_owned()),
                Ok(34) => msg.data_type = Some(r.read_string(bytes)?.to_owned()),
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
        + 1 + sizeof_len((&self.domain_id).len())
        + self.hashes.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.name.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.data_type.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.domain_id))?;
        for s in &self.hashes { w.write_with_tag(18, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.name { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.data_type { w.write_with_tag(34, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

