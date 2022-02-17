use bytes::Bytes;
use crate::core::uuid::Uuid;
pub use crate::database::Db;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::error::Error;
use std::sync::Arc;



#[derive(Debug, PartialEq, Eq)]
pub enum MemDbErrorTy {
    MsgNotFound
}
#[derive(Debug, PartialEq, Eq)]
pub struct MemDbError {
    pub error_ty: MemDbErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}
impl MemDbError {
    pub fn new(error_ty: MemDbErrorTy, file: &'static str, line: u32, msg: Option<String>) -> MemDbError {
        MemDbError {
            error_ty,
            file,
            line,
            msg
        }
    }
}
impl Display for MemDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self)
    }
}
impl Error for MemDbError { }

macro_rules! db_error {
    ($err:expr) => {
        MemDbError::new($err, file!(), line!(), None)
    };
    ($err_ty:expr, $error:expr) => {
        MemDbError::new($err, file!(), line!(), Some($error.to_string()))
    };
}

pub struct MemDb {
    msgs: BTreeMap<Arc<Uuid>, Bytes>,
    byte_size_data: BTreeMap<Arc<Uuid>, u32>
}
impl MemDb {
    pub fn new() -> MemDb {
        MemDb {
            msgs: BTreeMap::new(),
            byte_size_data: BTreeMap::new()
        }
    }
}
impl Db<MemDbError> for MemDb {
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), MemDbError> {
        self.msgs.insert(uuid.clone(), msg);
        self.byte_size_data.insert(uuid, msg_byte_size);
        Ok(())
    }
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, MemDbError> {
        match self.msgs.get(&uuid) {
            Some(msg) => Ok(msg.clone()),
            None => Err(db_error!(MemDbErrorTy::MsgNotFound))
        }
    }
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), MemDbError> {
        self.msgs.remove(&uuid);
        self.byte_size_data.remove(&uuid);
        Ok(())
    }
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, MemDbError> {
        Ok(vec![])
    }
}
