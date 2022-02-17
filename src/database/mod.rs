pub mod in_memory;
pub mod leveldb;

use bytes::Bytes;
use super::core::uuid::Uuid;
use std::error::Error;
use std::sync::Arc;

pub trait Db<E: Error>: Send + Sync {
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, E>;
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), E>;
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), E>;
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, E>;
}