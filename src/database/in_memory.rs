use bytes::Bytes;
use crate::core::uuid::Uuid;
pub use crate::database::Db;
use std::collections::BTreeMap;
use std::sync::Arc;

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
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), String> {
        self.msgs.insert(uuid.clone(), msg);
        self.byte_size_data.insert(uuid, msg_byte_size);
        Ok(())
    }
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, String> {
        match self.msgs.get(&uuid) {
            Some(msg) => Ok(msg.clone()),
            None => Err("msg not found".to_string())
        }
    }
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), String> {
        self.msgs.remove(&uuid);
        self.byte_size_data.remove(&uuid);
        Ok(())
    }
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, String> {
        Ok(vec![])
    }
}
