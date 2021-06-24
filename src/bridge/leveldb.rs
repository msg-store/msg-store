use crate::bridge::{Bridge, BridgeError};
use crate::error::ErrorCode;
use crate::msg_data::MsgData;
use crate::uuid::Uuid;
use bincode::{deserialize, serialize};
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{Options, ReadOptions, WriteOptions};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Serialize, Deserialize)]
pub struct LevelDBMessage {
    byte_size: u64,
    msg: Vec<u8>,
}
impl LevelDBMessage {
    pub fn new(msg: &Vec<u8>) -> LevelDBMessage {
        LevelDBMessage {
            byte_size: msg.len() as u64,
            msg: msg.to_owned(),
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, ErrorCode> {
        match serialize(&self) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(ErrorCode::new(0, "/serialize", &error.to_string())),
        }
    }

    pub fn from_bytes(bytes: &Vec<u8>) -> Result<LevelDBMessage, ErrorCode> {
        match deserialize::<LevelDBMessage>(bytes) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(ErrorCode::new(0, "/deserialize", &error.to_string())),
        }
    }
}

pub struct LevelDbBridge {
    messages: Database<Uuid>, // The LevelDB for messages
}

impl LevelDbBridge {
    pub fn new(db_path: &PathBuf) -> Result<LevelDbBridge, ErrorCode> {
        let mut options = Options::new();
        options.create_if_missing = true;
        let db = match Database::<Uuid>::open(&db_path, options) {
            Ok(database) => Ok(database),
            Err(error) => Err(BridgeError::Open(error.to_string()).to_error()),
        }?;
        Ok(LevelDbBridge { messages: db })
    }
}

impl Bridge for LevelDbBridge {
    fn post(&mut self, uuid: Rc<Uuid>, msg: &Vec<u8>) -> Result<(), ErrorCode> {
        let write_options = WriteOptions::new();
        let put_result = self.messages.put(write_options, uuid.clone(), &msg);
        match put_result {
            Ok(_) => Ok(()),
            Err(error) => Err(BridgeError::Post(error.to_string()).to_error()),
        }
    }

    fn put(&mut self, uuid: &Uuid, new_uuid: &Uuid) -> Result<(), ErrorCode> {
        let msg = match self.messages.get(ReadOptions::new(), uuid) {
            Ok(msg_option) => match msg_option {
                Some(msg) => Ok(msg),
                None => {
                    let error_msg = format!("Id not found");
                    Err(BridgeError::Put(error_msg).to_error())
                }
            },
            Err(error) => {
                let error_msg = error.to_string();
                Err(BridgeError::Put(error_msg).to_error())
            }
        }?;
        match self.messages.put(WriteOptions::new(), new_uuid, &msg) {
            Ok(_) => Ok(()),
            Err(error) => {
                let error_msg = error.to_string();
                Err(BridgeError::Put(error_msg).to_error())
            }
        }?;
        match self.messages.delete(WriteOptions::new(), uuid) {
            Ok(_) => Ok(()),
            Err(error) => {
                let error_msg = error.to_string();
                Err(BridgeError::Put(error_msg).to_error())
            }
        }?;
        Ok(())
    }

    fn get(&self, uuid: Rc<Uuid>) -> Result<Option<Vec<u8>>, ErrorCode> {
        let read_options = ReadOptions::new();
        match self.messages.get(read_options, uuid) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(BridgeError::Get(error.to_string()).to_error()),
        }
    }

    fn del(&mut self, uuid: Rc<Uuid>) -> Result<(), ErrorCode> {
        let write_options = WriteOptions::new();
        match self.messages.delete(write_options, uuid.clone()) {
            Ok(_) => Ok(()),
            Err(error) => Err(BridgeError::Del(error.to_string()).to_error()),
        }
    }

    fn read(&self) -> Result<Vec<MsgData>, ErrorCode> {
        let mut packets: Vec<MsgData> = vec![];
        let iterator = self.messages.iter(ReadOptions::new());
        for (uuid, serialized_msg) in iterator {
            let leveldb_msg_data = LevelDBMessage::from_bytes(&serialized_msg)?;
            let msg_data = MsgData::new(Rc::new(uuid), leveldb_msg_data.byte_size);
            packets.push(msg_data);
        }
        Ok(packets)
    }
}
