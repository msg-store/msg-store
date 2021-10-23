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
        fn should_prune_store_byte_size_to_10_when_store_max_byte_size_exists() {
            let mut store = Store::open();
            let first_packet = Packet::new(1, "1234567890".to_string());
            let second_packet = Packet::new(1, "1234567890".to_string());
            store.max_byte_size = Some(10);
            store.add(&first_packet).expect("Could not add first msg");
            store.add(&second_packet).expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_prune_store_byte_size_to_10_when_group_max_byte_size_exists() {
            let mut store = Store::open();
            let first_packet = Packet::new(1, "1234567890".to_string());
            let second_packet = Packet::new(1, "1234567890".to_string());
            store.add(&first_packet).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not find group");
            group.max_byte_size = Some(10);
            store.add(&second_packet).expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_prune_group_byte_size_to_10_when_group_max_byte_size_exists() {
            let mut store = Store::open();
            let first_packet = Packet::new(1, "1234567890".to_string());
            let second_packet = Packet::new(1, "1234567890".to_string());
            store.add(&first_packet).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            store.add(&second_packet).expect("Could not second msg");
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(group.byte_size, 10)
        }

        #[test]
        fn should_prune_oldest_msg_in_a_group_when_exceeding_group_max_byte_size() {
            let mut store = Store::open();
            let first_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let second_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_msg_in_a_group_when_exceeding_store_max_byte_size() {
            let mut store = Store::open();
            store.max_byte_size = Some(10);
            let first_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let second_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_lowest_pri_msg_in_the_store_when_exceeding_store_max_byte_size() {
            let mut store = Store::open();
            store.max_byte_size = Some(20);
            let first_uuid = store.add(&Packet::new(2, "1234567890".to_string())).expect("Could not add first msg");
            let second_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            let third_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            assert_eq!(Some(&2), store.id_to_group_map.get(&first_uuid));
            assert_eq!(None, store.id_to_group_map.get(&second_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&third_uuid));
        }

        #[test]
        fn should_return_msg_to_large_for_store_err() {
            let mut store = Store::open();
            store.max_byte_size = Some(9);
            let result = store.add(&Packet::new(2, "1234567890".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_to_large_for_group_err() {
            let mut store = Store::open();
            store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let result = store.add(&Packet::new(1, "12345678901".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_lacks_priority_err() {
            let mut store = Store::open();
            store.max_byte_size = Some(20);
            store.add(&Packet::new(2, "1234567890".to_string())).expect("Could not add first msg");
            store.add(&Packet::new(2, "1234567890".to_string())).expect("Could not second msg");
            let result = store.add(&Packet::new(1, "1234567890".to_string()));
            assert!(result.is_err());
        }

    }

    mod get {
        use crate::{
            database::mem::Store,
            store::Packet
        };

        #[test]
        fn should_return_msg() {
            let mut store = Store::open();
            let uuid = store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(Some(uuid), None).expect("Msg not found");
            assert_eq!(uuid, stored_packet.uuid);
            assert_eq!("first message", stored_packet.msg);
        }

        #[test]
        fn should_return_oldest_msg() {
            let mut store = Store::open();
            let first_uuid = store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            store.add(&Packet::new(1, "second message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(None, None).expect("Msg not found");
            assert_eq!(first_uuid, stored_packet.uuid);
            assert_eq!("first message", stored_packet.msg);
        }

    }

}