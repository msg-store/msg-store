pub mod in_memory;
pub mod leveldb;

use bytes::Bytes;
use super::core::uuid::Uuid;
use std::sync::Arc;

pub trait Db: Send + Sync {
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, String>;
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), String>;
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), String>;
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, String>;
}