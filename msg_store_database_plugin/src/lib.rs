use bytes::Bytes;
use msg_store_uuid::Uuid;
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
        match self {
            Self::CouldNotOpenDatabase |
            Self::CouldNotAddMsg |
            Self::CouldNotGetMsg |
            Self::CouldNotDeleteMsg |
            Self::CouldNotFetchData |
            Self::MsgNotFound => write!(f, "{}", self)
        }
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
        if let Some(msg) = &self.msg {
            write!(f, "DATABASE_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "DATABASE_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

pub trait Db: Send + Sync {
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, DatabaseError>;
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u64) -> Result<(), DatabaseError>;
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), DatabaseError>;
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u64)>, DatabaseError>;
}
