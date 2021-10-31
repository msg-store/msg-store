use bincode::{serialize, deserialize};
use crate::{
    Keeper,
    store::{
        GroupId,
        MsgByteSize,
        Package,
        PacketMetaData,
        Store
    },
    uuid::Uuid
};
use db_key::Key;
use leveldb::{
    database::Database,
    iterator::Iterable,
    kv::KV,
    options::{
        Options,
        ReadOptions,
        WriteOptions
    }
};
use serde::{Serialize, Deserialize};
use std::{
    fs::create_dir_all,
    path::Path
};

pub type LevelStore = Store<Leveldb>;

pub struct Leveldb {
    pub msgs: Database<Uuid>,
    pub data: Database<Uuid>
}

impl Leveldb {
    pub fn new(dir: &Path) -> Leveldb {
        create_dir_all(&dir).expect("Could not create db location dir.");

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

        let msgs = Database::open(msgs_path, msgs_options).expect("Could not open msgs database");
        let data = Database::open(Path::new(msg_data_path), msg_data_options).expect("Could not open data database");
        
        Leveldb {
            msgs,
            data
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct DbMetadata {
    priority: GroupId,
    byte_size: MsgByteSize
}

impl Keeper for Leveldb {
    fn add(&mut self, package: &Package) {
        let data = DbMetadata {
            priority: package.priority,
            byte_size: package.byte_size
        };
        let serialized_data = serialize(&data).expect("Could not serialize data");
        let msg = serialize(&package.msg).expect("Could not serialize msg");
        self.data.put(WriteOptions::new(), package.uuid, &serialized_data).expect("Could not insert metadata");
        self.msgs.put(WriteOptions::new(), package.uuid, &msg).expect("Could not insert msg");     
    }
    fn get(&mut self, uuid: &Uuid) -> Option<String> {
        match self.msgs.get(ReadOptions::new(), uuid).expect("Could not get msg") {
            Some(serialized_msg) => Some(deserialize(&serialized_msg).expect("Could not deserialize msg")),
            None => None
        }        
    }
    fn del(&mut self, uuid: &Uuid) {
        self.msgs.delete(WriteOptions::new(), uuid).expect("Could not delete msg");
    }
    fn fetch(&mut self) -> Vec<PacketMetaData> {
        self.data.value_iter(ReadOptions::new()).map(|data| {
            deserialize(&data).expect("Could not deserialize data")
        }).collect::<Vec<PacketMetaData>>()
    }
}

impl Key for Uuid {
    fn from_u8(key: &[u8]) -> Self {
        deserialize(key).expect("Could not deserialize key")
    }
    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&serialize(&self).expect("Could not serialize uuid"))
    }
}

pub fn open(location: &Path) -> LevelStore {
    Store::open(Leveldb::new(location))
}
