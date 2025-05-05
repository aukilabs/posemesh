// Automatically generated rust module for 'task.proto' file

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Code {
    OK = 200,
    Created = 201,
    Accepted = 202,
    BadRequest = 400,
}

impl Default for Code {
    fn default() -> Self {
        Code::OK
    }
}

impl From<i32> for Code {
    fn from(i: i32) -> Self {
        match i {
            200 => Code::OK,
            201 => Code::Created,
            202 => Code::Accepted,
            400 => Code::BadRequest,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Code {
    fn from(s: &'a str) -> Self {
        match s {
            "OK" => Code::OK,
            "Created" => Code::Created,
            "Accepted" => Code::Accepted,
            "BadRequest" => Code::BadRequest,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Status {
    PENDING = 0,
    STARTED = 1,
    DONE = 2,
    FAILED = 3,
    WAITING_FOR_RESOURCE = 4,
    RETRY = 5,
    PROCESSING = 6,
}

impl Default for Status {
    fn default() -> Self {
        Status::PENDING
    }
}

impl From<i32> for Status {
    fn from(i: i32) -> Self {
        match i {
            0 => Status::PENDING,
            1 => Status::STARTED,
            2 => Status::DONE,
            3 => Status::FAILED,
            4 => Status::WAITING_FOR_RESOURCE,
            5 => Status::RETRY,
            6 => Status::PROCESSING,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for Status {
    fn from(s: &'a str) -> Self {
        match s {
            "PENDING" => Status::PENDING,
            "STARTED" => Status::STARTED,
            "DONE" => Status::DONE,
            "FAILED" => Status::FAILED,
            "WAITING_FOR_RESOURCE" => Status::WAITING_FOR_RESOURCE,
            "RETRY" => Status::RETRY,
            "PROCESSING" => Status::PROCESSING,
            _ => Self::default(),
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Any {
    pub type_url: String,
    pub value: Vec<u8>,
}

impl<'a> MessageRead<'a> for Any {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.type_url = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.value = r.read_bytes(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Any {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.type_url).len())
        + 1 + sizeof_len((&self.value).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.type_url))?;
        w.write_with_tag(18, |w| w.write_bytes(&**&self.value))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct JobRequest {
    pub name: String,
    pub tasks: Vec<task::TaskRequest>,
    pub nonce: String,
    pub domain_id: String,
}

impl<'a> MessageRead<'a> for JobRequest {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.tasks.push(r.read_message::<task::TaskRequest>(bytes)?),
                Ok(26) => msg.nonce = r.read_string(bytes)?.to_owned(),
                Ok(34) => msg.domain_id = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for JobRequest {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + self.tasks.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + 1 + sizeof_len((&self.nonce).len())
        + 1 + sizeof_len((&self.domain_id).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.name))?;
        for s in &self.tasks { w.write_with_tag(18, |w| w.write_message(s))?; }
        w.write_with_tag(26, |w| w.write_string(&**&self.nonce))?;
        w.write_with_tag(34, |w| w.write_string(&**&self.domain_id))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Job {
    pub id: String,
    pub name: String,
    pub tasks: Vec<task::Task>,
}

impl<'a> MessageRead<'a> for Job {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.id = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(26) => msg.tasks.push(r.read_message::<task::Task>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Job {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.id).len())
        + 1 + sizeof_len((&self.name).len())
        + self.tasks.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.id))?;
        w.write_with_tag(18, |w| w.write_string(&**&self.name))?;
        for s in &self.tasks { w.write_with_tag(26, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct SubmitJobResponse {
    pub code: task::Code,
    pub job_id: String,
    pub err_msg: String,
}

impl<'a> MessageRead<'a> for SubmitJobResponse {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.code = r.read_enum(bytes)?,
                Ok(18) => msg.job_id = r.read_string(bytes)?.to_owned(),
                Ok(26) => msg.err_msg = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for SubmitJobResponse {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.code) as u64)
        + 1 + sizeof_len((&self.job_id).len())
        + 1 + sizeof_len((&self.err_msg).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(8, |w| w.write_enum(*&self.code as i32))?;
        w.write_with_tag(18, |w| w.write_string(&**&self.job_id))?;
        w.write_with_tag(26, |w| w.write_string(&**&self.err_msg))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TaskRequest {
    pub name: String,
    pub capability_filters: task::CapabilityFilters,
    pub max_budget: Option<u64>,
    pub timeout: String,
    pub needs: Vec<String>,
    pub resource_recruitment: task::ResourceRecruitment,
    pub sender: String,
    pub receiver: Option<String>,
    pub data: Option<task::Any>,
}

impl<'a> MessageRead<'a> for TaskRequest {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(18) => msg.capability_filters = r.read_message::<task::CapabilityFilters>(bytes)?,
                Ok(24) => msg.max_budget = Some(r.read_uint64(bytes)?),
                Ok(34) => msg.timeout = r.read_string(bytes)?.to_owned(),
                Ok(42) => msg.needs.push(r.read_string(bytes)?.to_owned()),
                Ok(50) => msg.resource_recruitment = r.read_message::<task::ResourceRecruitment>(bytes)?,
                Ok(58) => msg.sender = r.read_string(bytes)?.to_owned(),
                Ok(66) => msg.receiver = Some(r.read_string(bytes)?.to_owned()),
                Ok(74) => msg.data = Some(r.read_message::<task::Any>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for TaskRequest {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + 1 + sizeof_len((&self.capability_filters).get_size())
        + self.max_budget.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + 1 + sizeof_len((&self.timeout).len())
        + self.needs.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + 1 + sizeof_len((&self.resource_recruitment).get_size())
        + 1 + sizeof_len((&self.sender).len())
        + self.receiver.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + self.data.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.name))?;
        w.write_with_tag(18, |w| w.write_message(&self.capability_filters))?;
        if let Some(ref s) = self.max_budget { w.write_with_tag(24, |w| w.write_uint64(*s))?; }
        w.write_with_tag(34, |w| w.write_string(&**&self.timeout))?;
        for s in &self.needs { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        w.write_with_tag(50, |w| w.write_message(&self.resource_recruitment))?;
        w.write_with_tag(58, |w| w.write_string(&**&self.sender))?;
        if let Some(ref s) = self.receiver { w.write_with_tag(66, |w| w.write_string(&**s))?; }
        if let Some(ref s) = self.data { w.write_with_tag(74, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct CapabilityFilters {
    pub endpoint: String,
    pub min_gpu: Option<i32>,
    pub min_cpu: Option<i32>,
}

impl<'a> MessageRead<'a> for CapabilityFilters {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.endpoint = r.read_string(bytes)?.to_owned(),
                Ok(16) => msg.min_gpu = Some(r.read_int32(bytes)?),
                Ok(24) => msg.min_cpu = Some(r.read_int32(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for CapabilityFilters {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.endpoint).len())
        + self.min_gpu.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
        + self.min_cpu.as_ref().map_or(0, |m| 1 + sizeof_varint(*(m) as u64))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.endpoint))?;
        if let Some(ref s) = self.min_gpu { w.write_with_tag(16, |w| w.write_int32(*s))?; }
        if let Some(ref s) = self.min_cpu { w.write_with_tag(24, |w| w.write_int32(*s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ResourceRecruitment {
    pub recruitment_policy: task::mod_ResourceRecruitment::RecruitmentPolicy,
    pub termination_policy: task::mod_ResourceRecruitment::TerminationPolicy,
}

impl<'a> MessageRead<'a> for ResourceRecruitment {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.recruitment_policy = r.read_enum(bytes)?,
                Ok(16) => msg.termination_policy = r.read_enum(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ResourceRecruitment {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_varint(*(&self.recruitment_policy) as u64)
        + 1 + sizeof_varint(*(&self.termination_policy) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(8, |w| w.write_enum(*&self.recruitment_policy as i32))?;
        w.write_with_tag(16, |w| w.write_enum(*&self.termination_policy as i32))?;
        Ok(())
    }
}

pub mod mod_ResourceRecruitment {


#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RecruitmentPolicy {
    ALWAYS = 0,
    IF_NOT_PRESENT = 1,
    NEVER = 2,
    FAIL = 3,
}

impl Default for RecruitmentPolicy {
    fn default() -> Self {
        RecruitmentPolicy::ALWAYS
    }
}

impl From<i32> for RecruitmentPolicy {
    fn from(i: i32) -> Self {
        match i {
            0 => RecruitmentPolicy::ALWAYS,
            1 => RecruitmentPolicy::IF_NOT_PRESENT,
            2 => RecruitmentPolicy::NEVER,
            3 => RecruitmentPolicy::FAIL,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for RecruitmentPolicy {
    fn from(s: &'a str) -> Self {
        match s {
            "ALWAYS" => RecruitmentPolicy::ALWAYS,
            "IF_NOT_PRESENT" => RecruitmentPolicy::IF_NOT_PRESENT,
            "NEVER" => RecruitmentPolicy::NEVER,
            "FAIL" => RecruitmentPolicy::FAIL,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TerminationPolicy {
    KEEP = 0,
    TERMINATE = 1,
}

impl Default for TerminationPolicy {
    fn default() -> Self {
        TerminationPolicy::KEEP
    }
}

impl From<i32> for TerminationPolicy {
    fn from(i: i32) -> Self {
        match i {
            0 => TerminationPolicy::KEEP,
            1 => TerminationPolicy::TERMINATE,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for TerminationPolicy {
    fn from(s: &'a str) -> Self {
        match s {
            "KEEP" => TerminationPolicy::KEEP,
            "TERMINATE" => TerminationPolicy::TERMINATE,
            _ => Self::default(),
        }
    }
}

}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ConsumeDataInputV1 {
    pub query: domain_data::Query,
    pub keep_alive: bool,
}

impl<'a> MessageRead<'a> for ConsumeDataInputV1 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.query = r.read_message::<domain_data::Query>(bytes)?,
                Ok(16) => msg.keep_alive = r.read_bool(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for ConsumeDataInputV1 {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.query).get_size())
        + 1 + sizeof_varint(*(&self.keep_alive) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_message(&self.query))?;
        w.write_with_tag(16, |w| w.write_bool(*&self.keep_alive))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct StoreDataOutputV1 {
    pub ids: Vec<String>,
}

impl<'a> MessageRead<'a> for StoreDataOutputV1 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.ids.push(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for StoreDataOutputV1 {
    fn get_size(&self) -> usize {
        0
        + self.ids.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.ids { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LocalRefinementOutputV1 {
    pub result_ids: Vec<String>,
}

impl<'a> MessageRead<'a> for LocalRefinementOutputV1 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.result_ids.push(r.read_string(bytes)?.to_owned()),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LocalRefinementOutputV1 {
    fn get_size(&self) -> usize {
        0
        + self.result_ids.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.result_ids { w.write_with_tag(10, |w| w.write_string(&**s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Task {
    pub name: String,
    pub receiver: Option<String>,
    pub endpoint: String,
    pub access_token: Option<String>,
    pub job_id: String,
    pub sender: String,
    pub status: task::Status,
    pub output: Option<task::Any>,
}

impl<'a> MessageRead<'a> for Task {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(18) => msg.name = r.read_string(bytes)?.to_owned(),
                Ok(26) => msg.receiver = Some(r.read_string(bytes)?.to_owned()),
                Ok(34) => msg.endpoint = r.read_string(bytes)?.to_owned(),
                Ok(42) => msg.access_token = Some(r.read_string(bytes)?.to_owned()),
                Ok(50) => msg.job_id = r.read_string(bytes)?.to_owned(),
                Ok(58) => msg.sender = r.read_string(bytes)?.to_owned(),
                Ok(72) => msg.status = r.read_enum(bytes)?,
                Ok(82) => msg.output = Some(r.read_message::<task::Any>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Task {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.name).len())
        + self.receiver.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + 1 + sizeof_len((&self.endpoint).len())
        + self.access_token.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
        + 1 + sizeof_len((&self.job_id).len())
        + 1 + sizeof_len((&self.sender).len())
        + 1 + sizeof_varint(*(&self.status) as u64)
        + self.output.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(18, |w| w.write_string(&**&self.name))?;
        if let Some(ref s) = self.receiver { w.write_with_tag(26, |w| w.write_string(&**s))?; }
        w.write_with_tag(34, |w| w.write_string(&**&self.endpoint))?;
        if let Some(ref s) = self.access_token { w.write_with_tag(42, |w| w.write_string(&**s))?; }
        w.write_with_tag(50, |w| w.write_string(&**&self.job_id))?;
        w.write_with_tag(58, |w| w.write_string(&**&self.sender))?;
        w.write_with_tag(72, |w| w.write_enum(*&self.status as i32))?;
        if let Some(ref s) = self.output { w.write_with_tag(82, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct LocalRefinementInputV1 {
    pub query: domain_data::Query,
}

impl<'a> MessageRead<'a> for LocalRefinementInputV1 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.query = r.read_message::<domain_data::Query>(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for LocalRefinementInputV1 {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.query).get_size())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_message(&self.query))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct DomainClusterHandshake {
    pub access_token: String,
}

impl<'a> MessageRead<'a> for DomainClusterHandshake {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.access_token = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for DomainClusterHandshake {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.access_token).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.access_token))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct GlobalRefinementInputV1 {
    pub local_refinement_results: Vec<task::LocalRefinementOutputV1>,
}

impl<'a> MessageRead<'a> for GlobalRefinementInputV1 {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.local_refinement_results.push(r.read_message::<task::LocalRefinementOutputV1>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for GlobalRefinementInputV1 {
    fn get_size(&self) -> usize {
        0
        + self.local_refinement_results.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.local_refinement_results { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Error {
    pub message: String,
}

impl<'a> MessageRead<'a> for Error {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.message = r.read_string(bytes)?.to_owned(),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for Error {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.message).len())
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_string(&**&self.message))?;
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TaskHandler {
    pub task: task::Task,
    pub dependencies: KVMap<String, bool>,
    pub job_id: String,
    pub err_msg: String,
    pub retries: u32,
    pub updated_at: u64,
    pub created_at: u64,
}

impl<'a> MessageRead<'a> for TaskHandler {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.task = r.read_message::<task::Task>(bytes)?,
                Ok(18) => {
                    let (key, value) = r.read_map(bytes, |r, bytes| Ok(r.read_string(bytes)?.to_owned()), |r, bytes| Ok(r.read_bool(bytes)?))?;
                    msg.dependencies.insert(key, value);
                }
                Ok(26) => msg.job_id = r.read_string(bytes)?.to_owned(),
                Ok(34) => msg.err_msg = r.read_string(bytes)?.to_owned(),
                Ok(40) => msg.retries = r.read_uint32(bytes)?,
                Ok(48) => msg.updated_at = r.read_uint64(bytes)?,
                Ok(56) => msg.created_at = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for TaskHandler {
    fn get_size(&self) -> usize {
        0
        + 1 + sizeof_len((&self.task).get_size())
        + self.dependencies.iter().map(|(k, v)| 1 + sizeof_len(2 + sizeof_len((k).len()) + sizeof_varint(*(v) as u64))).sum::<usize>()
        + 1 + sizeof_len((&self.job_id).len())
        + 1 + sizeof_len((&self.err_msg).len())
        + 1 + sizeof_varint(*(&self.retries) as u64)
        + 1 + sizeof_varint(*(&self.updated_at) as u64)
        + 1 + sizeof_varint(*(&self.created_at) as u64)
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(10, |w| w.write_message(&self.task))?;
        for (k, v) in self.dependencies.iter() { w.write_with_tag(18, |w| w.write_map(2 + sizeof_len((k).len()) + sizeof_varint(*(v) as u64), 10, |w| w.write_string(&**k), 16, |w| w.write_bool(*v)))?; }
        w.write_with_tag(26, |w| w.write_string(&**&self.job_id))?;
        w.write_with_tag(34, |w| w.write_string(&**&self.err_msg))?;
        w.write_with_tag(40, |w| w.write_uint32(*&self.retries))?;
        w.write_with_tag(48, |w| w.write_uint64(*&self.updated_at))?;
        w.write_with_tag(56, |w| w.write_uint64(*&self.created_at))?;
        Ok(())
    }
}

