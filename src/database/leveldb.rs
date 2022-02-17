use bincode::{serialize, deserialize};
use bytes::Bytes;
use crate::core::uuid::Uuid;
pub use crate::database::Db;
use db_key::Key;
use leveldb::database::Database;
use leveldb::iterator::Iterable;
use leveldb::kv::KV;
use leveldb::options::{
    Options,
    ReadOptions,
    WriteOptions
};
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::sync::Arc;

#[derive(Deserialize, Serialize)]
pub struct Id(Vec<u8>);

impl Key for Id {
    fn from_u8(key: &[u8]) -> Self {
        // Id(key.to_vec())
        deserialize(key).expect("Could not deserialize key")
    }
    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        // f(&self.0.to_vec())
        f(&serialize(&self).expect("Could not serialize uuid"))
    }
}

pub struct Leveldb {
    pub msgs: Database<Id>,
    pub data: Database<Id>
}

impl Leveldb {
    pub fn new(dir: &Path) -> Result<Leveldb, String> {

        let mut msgs_path = dir.to_path_buf();
        msgs_path.push("msgs");
        let msgs_path = msgs_path.as_path();

        let mut msg_data_path = dir.to_path_buf();
        msg_data_path.push("msg_data");
        let msg_data_path = msg_data_path.as_path();

        let mut msgs_options = Options::new();
        msgs_options.create_if_missing = true;

        let mut msg_data_options = Options::new();
        msg_data_options.create_if_missing = true;

        let msgs = match Database::open(msgs_path, msgs_options) {
            Ok(msgs) => msgs,
            Err(error) => {
                return Err(format!("PLUGIN_LEVEL_DB_ERROR_CODE: 034d85df-3e0e-4522-953c-af31cb6a7550. Could not open msgs database: {}", error.to_string()));
            }
        };
        let data = match Database::open(Path::new(msg_data_path), msg_data_options){
            Ok(data) => data,
            Err(error) => {
                return Err(format!("PLUGIN_LEVEL_DB_ERROR_CODE: ef9cb8ae-8775-4cfc-8725-074e762287aa. Could not open msgs database: {}", error.to_string()));
            } 
        };
        
        Ok(Leveldb {
            msgs,
            data
        })
    }
}

impl Db for Leveldb {
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u32) -> Result<(), String> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        self.data.put(WriteOptions::new(), Id(uuid_bytes.clone()), format!("{}", msg_byte_size).as_bytes()).expect("Could not insert metadata");
        self.msgs.put(WriteOptions::new(), Id(uuid_bytes), &msg).expect("Could not insert msg");
        Ok(())
    }
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, String> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        match self.msgs.get(ReadOptions::new(), Id(uuid_bytes)).expect("Could not get msg") {
            Some(msg) => Ok(Bytes::copy_from_slice(&msg)),
            None => Err("Message not found".to_string())
        }
    }
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), String> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        self.msgs.delete(WriteOptions::new(), Id(uuid_bytes.clone())).expect("Could not delete msg");
        self.data.delete(WriteOptions::new(), Id(uuid_bytes)).expect("Could not delete data");
        Ok(())
    }
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u32)>, String> {
        self.data.iter(ReadOptions::new()).map(|(id, data)| {
            let data = match String::from_utf8(data) {
                Ok(data) => data,
                Err(_error) => {
                    return Err("Could not convert bytes into String".to_string());
                }
            };
            let data = match data.parse::<u32>() {
                Ok(data) => data,
                Err(_error) => {
                    return Err("Could not parse u32".to_string());
                }
            };
            let uuid = match String::from_utf8(id.0) {
                Ok(uuid) => uuid,
                Err(_error) => {
                    return Err("Could not convert bytes into Uuid String".to_string())
                }
            };
            let uuid = match Uuid::from_string(&uuid) {
                Ok(uuid) => uuid,
                Err(error) => {
                    return Err(format!("{:#?}", error))
                }
            };
            Ok((uuid, data))
        }).collect::<Result<Vec<(Arc<Uuid>, u32)>, String>>()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use crate::core::uuid::Uuid;
    use crate::database::Db;
    use crate::database::leveldb::Leveldb;
    use std::fs::{create_dir_all, remove_dir_all};
    use std::path::{Path, PathBuf};
    use std::str::FromStr;

    fn dir_setup(tmp_dir: &Path) {
        if tmp_dir.exists() {
            remove_dir_all(&tmp_dir).unwrap();
        }
        create_dir_all(&tmp_dir).unwrap();
    }

    fn dir_teardown(tmp_dir: &Path) {
        if tmp_dir.exists() {
            remove_dir_all(&tmp_dir).unwrap();
        }
    }

    #[test]
    fn it_works() {
        // app  setup
        let tmp_dir = PathBuf::from_str(&format!("/tmp/msg-store-plugin-leveldb")).unwrap();
        dir_setup(&tmp_dir);

        // create a fake uuid and message
        let uuid = Uuid::from_string("1-0-1-0").unwrap();
        let inner_msg = b"my message";
        let msg = Bytes::copy_from_slice(inner_msg);
        let msg_byte_size = inner_msg.len() as u32;
        {
            // get level instance
            // add one message
            // force out of scope
            let mut level = Leveldb::new(&tmp_dir).unwrap();
            level.add(uuid.clone(), msg.clone(), msg_byte_size).unwrap();
        }
        // get level instance
        let mut level = Leveldb::new(&tmp_dir).unwrap();
        
        // fetch messages
        let msgs = level.fetch().unwrap();
        assert_eq!(1, msgs.len());
        let (received_uuid, received_msg_byte_size) = &msgs[0];
        assert_eq!(uuid, received_uuid.clone());
        assert_eq!(msg_byte_size, *received_msg_byte_size);

        // get msg
        let received_msg = level.get(uuid.clone()).unwrap();
        assert_eq!(msg, received_msg);

        // delete msg
        level.del(uuid.clone()).unwrap();

        let get_msg_result = level.get(uuid);
        assert!(get_msg_result.is_err());

        
        // assert_eq!(2 + 2, 4);
        dir_teardown(&tmp_dir);
    }
}
