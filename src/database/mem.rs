use crate::{
    Keeper,
    store::{
        Package,
        PacketMetaData,
        Store as BaseStore
    },
    uuid::Uuid
};
use std::collections::BTreeMap;

pub type Store = BaseStore<MemDb>;

pub struct MemDb {
    msgs: BTreeMap<Uuid, String>
}

impl MemDb {
    pub fn new() -> MemDb {
        MemDb {
            msgs: BTreeMap::new()
        }
    }
}

impl Keeper for MemDb {
    fn add(&mut self, package: &Package) {
        self.msgs.insert(package.uuid, package.msg.clone());
    }
    fn get(&mut self, uuid: &Uuid) -> Option<String> {
        match self.msgs.get(uuid) {
            Some(msg) => Some(msg.clone()),
            None => None
        }
    }
    fn del(&mut self, uuid: &Uuid) {
        self.msgs.remove(uuid);
    }
    fn fetch(&mut self) -> Vec<PacketMetaData> {
        vec![]
    }
}

impl BaseStore<MemDb> {
    pub fn open() -> BaseStore<MemDb> {
        Self::new(MemDb::new())
    }
}

#[cfg(test)]
mod tests {

    mod add {
        use crate::{
            database::mem::Store,
            store::Packet
        };

        #[test]
        fn should_increase_store_byte_size() {
            let mut store = Store::open();
            let packet = Packet::new(1, "1234567890".to_string());
            store.add(&packet).expect("Could not add msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_increase_group_byte_size() {
            let mut store = Store::open();
            let packet = Packet::new(1, "1234567890".to_string());
            store.add(&packet).expect("Could not add msg");
            let group = store.groups_map.get(&1).expect("Could not find group");
            assert_eq!(group.byte_size, 10)
        }

        #[test]
        fn should_prune_byte_size_to_10() {
            let mut store = Store::open();
            let first_packet = Packet::new(1, "1234567890".to_string());
            let second_packet = Packet::new(1, "1234567890".to_string());
            store.max_byte_size = Some(10);
            store.add(&first_packet).expect("Could not add first msg");
            store.add(&second_packet).expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

    }

}