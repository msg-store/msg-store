pub mod in_memory;
pub mod leveldb;

use bytes::Bytes;
use super::core::uuid::Uuid;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Debug)]
pub enum DatabaseErrorTy {
    CouldNotOpenDatabase,
    CouldNotAddMsg,
    CouldNotGetMsg,
    CouldNotDeleteMsg,
    CouldNotFetchData,
    MsgNotFound
}
impl Display for DatabaseErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug)]
pub struct DatabaseError {
    pub err_ty: DatabaseErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}
impl Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DATABASE_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)?;
        if let Some(msg) = &self.msg {
            write!(f, "{}", msg)
        } else {
            Ok(())
        }
    }   
}

pub trait Db: Send + Sync {
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, DatabaseError>;
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), DatabaseError>;
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), DatabaseError>;
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, DatabaseError>;
}