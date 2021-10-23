use bincode::{serialize, deserialize};
use crate::{
    Keeper,
    store::{
        GroupId,
        MsgByteSize,
        Package,
        PacketMetaData
    },
    uuid::Uuid
};
use db_key::Key;
use leveldb::{
    database::Database,
    iterator::Iterable,
    kv::KV,
    options::{
        ReadOptions,
        WriteOptions
    }
};
use serde::{Serialize, Deserialize};

pub struct Leveldb {
    pub msgs: Database<Uuid>,
    pub data: Database<Uuid>
}

impl Leveldb {
    
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
