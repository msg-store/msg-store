use bytes::Bytes;
use crate::core::uuid::Uuid;
pub use crate::database::{Db, DatabaseError, DatabaseErrorTy};
use std::collections::BTreeMap;
use std::sync::Arc;

macro_rules! memdb_error {
    ($err_ty:expr) => {
        DatabaseError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    // ($err_ty:expr, $msg:expr) => {
    //     DatabaseError {
    //         err_ty: $err_ty,
    //         file: file!(),
    //         line: line!(),
    //         msg: Some($msg.to_string())
    //     }
    // };
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
impl Db for MemDb {
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), DatabaseError> {
        self.msgs.insert(uuid.clone(), msg);
        self.byte_size_data.insert(uuid, msg_byte_size);
        Ok(())
    }
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, DatabaseError> {
        match self.msgs.get(&uuid) {
            Some(msg) => Ok(msg.clone()),
            None => Err(memdb_error!(DatabaseErrorTy::MsgNotFound))
        }
    }
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), DatabaseError> {
        self.msgs.remove(&uuid);
        self.byte_size_data.remove(&uuid);
        Ok(())
    }
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, DatabaseError> {
        Ok(vec![])
    }
}
