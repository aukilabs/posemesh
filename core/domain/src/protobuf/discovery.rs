// Automatically generated rust module for 'discovery.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::*;

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Capability {
    pub endpoint: String,
    pub capacity: i32,
}

impl<'a> MessageRead<'a> for Capability {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.endpoint = r.read_string(bytes)?.to_owned(),
                Ok(16) => msg.capacity = r.read_int32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Capability {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.endpoint).len())
        + 1 + sizeof_varint(*(&self.capacity) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.endpoint))?;
        w.write_with_tag(16, |w| w.write_int32(*&self.capacity))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Node {
    pub id: String,
    pub capabilities: Vec<discovery::Capability>,
    pub name: String,
}

impl<'a> MessageRead<'a> for Node {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.capabilities.push(r.read_message::<discovery::Capability>(bytes)?),
                Ok(26) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Node {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.id).len())
        + self.capabilities.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + 1 + sizeof_len((&self.name).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.id))?;
        for s in &self.capabilities { w.write_with_tag(18, |w| w.write_message(s))?; }
        w.write_with_tag(26, |w| w.write_string(&**&self.name))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct JoinClusterRequest {
    pub node: discovery::Node,
}

impl<'a> MessageRead<'a> for JoinClusterRequest {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.node = r.read_message::<discovery::Node>(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for JoinClusterRequest {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.node).get_size())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_message(&self.node))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct JoinClusterResponse { }

impl<'a> MessageRead<'a> for JoinClusterResponse {
    fn from_reader(r: &mut BytesReader, _: &[u8]) -> Result<Self> {
        r.read_to_end();
        Ok(Self::default())
    }
}

impl MessageWrite for JoinClusterResponse { }

