use bincode::{serialize, deserialize};
use bytes::Bytes;
use msg_store_uuid::Uuid;
pub use database_plugin::{Db, DatabaseError, DatabaseErrorTy};
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
use std::fs::create_dir_all;
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

macro_rules! leveldb_error {
    ($err_ty:expr) => {
        DatabaseError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        DatabaseError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub struct Leveldb {
    pub msgs: Database<Id>,
    pub data: Database<Id>
}

impl Leveldb {
    pub fn new(dir: &Path) -> Result<Leveldb, DatabaseError> {

        if !dir.exists() {
            if let Err(err) = create_dir_all(&dir) {
                return Err(leveldb_error!(DatabaseErrorTy::CouldNotOpenDatabase, err))
            }
        }

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
            Ok(msgs) => Ok(msgs),
            Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotOpenDatabase, error))
        }?;
        let data = match Database::open(Path::new(msg_data_path), msg_data_options){
            Ok(data) => Ok(data),
            Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotOpenDatabase, error)) 
        }?;
        
        Ok(Leveldb {
            msgs,
            data
        })
    }
}

impl Db for Leveldb {
    fn add(&mut self, uuid: Arc<Uuid>, msg: Bytes, msg_byte_size: u64) -> Result<(), DatabaseError> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        let byte_size_str = msg_byte_size.to_string();
        let byte_size = byte_size_str.as_bytes();
        if let Err(error) = self.data.put(WriteOptions::new(), Id(uuid_bytes.clone()), byte_size) {
            return Err(leveldb_error!(DatabaseErrorTy::CouldNotAddMsg, error))
        };
        if let Err(error) = self.msgs.put(WriteOptions::new(), Id(uuid_bytes), &msg) {
            return Err(leveldb_error!(DatabaseErrorTy::CouldNotAddMsg, error))
        };
        Ok(())
    }
    fn get(&mut self, uuid: Arc<Uuid>) -> Result<Bytes, DatabaseError> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        let msg_option = match self.msgs.get(ReadOptions::new(), Id(uuid_bytes)) {
            Ok(msg_option) => Ok(msg_option),
            Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotGetMsg, error))
        }?;
        match msg_option {
            Some(msg) => Ok(Bytes::copy_from_slice(&msg)),
            None => Err(leveldb_error!(DatabaseErrorTy::MsgNotFound))
        }
    }
    fn del(&mut self, uuid: Arc<Uuid>) -> Result<(), DatabaseError> {
        let uuid_bytes = uuid.to_string().as_bytes().to_vec();
        if let Err(error) = self.msgs.delete(WriteOptions::new(), Id(uuid_bytes.clone())) {
            return Err(leveldb_error!(DatabaseErrorTy::CouldNotDeleteMsg, error))
        };
        if let Err(error) = self.data.delete(WriteOptions::new(), Id(uuid_bytes)) {
            return Err(leveldb_error!(DatabaseErrorTy::CouldNotDeleteMsg, error))
        };
        Ok(())
    }
    fn fetch(&mut self) -> Result<Vec<(Arc<Uuid>, u64)>, DatabaseError> {
        self.data.iter(ReadOptions::new()).map(|(id, data)| {
            let data = match String::from_utf8(data) {
                Ok(data) => Ok(data),
                Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotFetchData, error))
            }?;
            let data = match data.parse::<u64>() {
                Ok(data) => Ok(data),
                Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotFetchData, error))
            }?;
            let uuid = match String::from_utf8(id.0) {
                Ok(uuid) => Ok(uuid),
                Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotFetchData, error))
            }?;
            let uuid = match Uuid::from_string(&uuid) {
                Ok(uuid) => Ok(uuid),
                Err(error) => Err(leveldb_error!(DatabaseErrorTy::CouldNotFetchData, error))
            }?;
            Ok((uuid, data))
        }).collect::<Result<Vec<(Arc<Uuid>, u64)>, DatabaseError>>()
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use msg_store_uuid::Uuid;
    use database_plugin::Db;
    use crate::Leveldb;
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
        let msg_byte_size = inner_msg.len() as u64;
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
