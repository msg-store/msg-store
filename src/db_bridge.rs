use bincode::{deserialize, serialize};
use crate::msg_data::MsgData;
use crate::uuid::Uuid;
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{Options,WriteOptions,ReadOptions};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug)]
pub enum DbError {
    Write(String),
    Read(String),
    Open(String),
    IdNotFound,
    MsgNotFound,
    Other(String)
}

pub trait Bridge {
    fn put(&mut self, uuid: Rc<Uuid>, msg: &Vec<u8>) -> Result<(), DbError>;
    fn get(&self, uuid: Rc<Uuid>) -> Result<Option<Vec<u8>>, DbError>;
    fn del(&mut self, uuid: Rc<Uuid>) -> Result<(), DbError>;
    fn read(&self) -> Result<Vec<MsgData>, DbError>;
}

pub struct LevelDbBridge {
  msg_size_storage: Database<Uuid>,  // The LevelDB for message sizes
  message_storage: Database<Uuid>    // The LevelDB for messages
}

impl LevelDbBridge {

    pub fn new(dir: &PathBuf) -> Result<LevelDbBridge, DbError> {
        // Get the msg size database path
        // * Init the path by cloning
        // * Append the name of the database
        let mut msg_size_storage_path = dir.clone();
        msg_size_storage_path.push("id_storage");
        // Get the msg storage database path
        // * Init the path by cloning
        // * Append the name of the database
        let mut message_storage_path = dir.clone();
        message_storage_path.push("message_storage");
        // Open the msg size storage database
        // * Set default options for opening the database
        // * Open the database
        let mut options = Options::new();
        options.create_if_missing = true;
        let msg_size_storage = match Database::<Uuid>::open(&msg_size_storage_path, options) {
            Ok(database) => Ok(database),
            Err(error) => Err(DbError::Open(error.to_string()))
        }?;
        // Open the msg_storage database
        // * Set default options for opening the database
        // * Open the database
        let mut options = Options::new();
        options.create_if_missing = true;
        let message_storage = match Database::<Uuid>::open(&message_storage_path, options) {
            Ok(database) => Ok(database),
            Err(error) => Err(DbError::Open(error.to_string()))
        }?;
        // return the bridge and the vector of uuids
        Ok(
            LevelDbBridge {
                msg_size_storage,
                message_storage,
            }
        )
    }

}

impl Bridge for LevelDbBridge {

    fn put(&mut self, uuid: Rc<Uuid>, msg: &Vec<u8>) -> Result<(), DbError> {
        let write_options = WriteOptions::new();
        let msg_size = serialize(&msg.len()).unwrap();
        let put_result = self.msg_size_storage.put(write_options, uuid.clone(), &msg_size);
        match put_result {
            Ok(_) => Ok(()),
            Err(error) => Err(DbError::Write(error.to_string()))
        }?;
        let put_result = self.message_storage.put(write_options, uuid, msg);
        match put_result {
            Ok(_) => Ok(()),
            Err(error) => Err(DbError::Write(error.to_string()))
        }?;
        Ok(())
    }

    fn get(&self, uuid: Rc<Uuid>) -> Result<Option<Vec<u8>>, DbError> {
        let read_options = ReadOptions::new();
        let get_result = self.message_storage.get(read_options, uuid);
        match get_result {
            Ok(msg) => Ok(msg),
            Err(error) => Err(DbError::Read(error.to_string()))
        }
     }

    fn del(&mut self, uuid: Rc<Uuid>) -> Result<(), DbError> {
        let write_options = WriteOptions::new();
        let delete_result = self.msg_size_storage.delete(write_options, uuid.clone());
        match delete_result {
            Ok(_) => Ok(()),
            Err(error) => Err(DbError::Read(error.to_string()))
        }?;
        let delete_result = self.message_storage.delete(write_options, uuid.clone());
        match delete_result {
            Ok(_) => Ok(()),
            Err(error) => Err(DbError::Read(error.to_string()))
        }?;
        Ok(())
     }
    
    fn read(&self) -> Result<Vec<MsgData>, DbError> {
        let mut packets: Vec<MsgData> = vec![];
        let iterator = self.msg_size_storage.iter(ReadOptions::new());
        for (uuid, byte_size) in iterator {
            let byte_size = match deserialize(&byte_size) {
                Ok(byte_size) => byte_size,
                Err(error) => {
                    return Err(DbError::Other(error.to_string()))
                }
            };
            let msg_data = MsgData::new(Rc::new(uuid), byte_size);
            packets.push(msg_data);
        }
        Ok(packets)
     }

}

