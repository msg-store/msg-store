use crate::{
    Keeper,
    store::{
        Package,
        PacketMetaData,
        Store
    },
    uuid::Uuid
};
use std::collections::BTreeMap;

/// This is a store which utilizes an in memory database.
/// 
/// The database is simply a BTreeMap.
/// 
/// This Store also comes with an associated function called "open".
pub type MemStore = Store<MemDb>;

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

pub fn open() -> MemStore {
    Store::open(MemDb::new())
}

#[cfg(test)]
mod tests {

    mod add {
        use crate::{
            mem::open,
            store::{ Packet, GroupDefaults }
        };

        #[test]
        fn should_increase_store_byte_size() {
            let mut store = open();
            let packet = Packet::new(1, "1234567890".to_string());
            store.add(&packet).expect("Could not add msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_increase_group_byte_size() {
            let mut store = open();
            let packet = Packet::new(1, "1234567890".to_string());
            store.add(&packet).expect("Could not add msg");
            let group = store.groups_map.get(&1).expect("Could not find group");
            assert_eq!(group.byte_size, 10)
        }

        #[test]
        fn should_increase_inserted_msg_count() {
            let mut store = open();
            assert_eq!(0, store.msgs_inserted);
            let packet = Packet::new(1, "1234567890".to_string());
            store.add(&packet).expect("Could not add msg");
            assert_eq!(1, store.msgs_inserted);
        }

        #[test]
        fn should_prune_store_byte_size_to_10_when_store_max_byte_size_exists() {
            let mut store = open();
            let first_packet = Packet::new(1, "1234567890".to_string());
            let second_packet = Packet::new(1, "1234567890".to_string());
            store.max_byte_size = Some(10);
            store.add(&first_packet).expect("Could not add first msg");
            store.add(&second_packet).expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_prune_store_byte_size_to_10_when_group_max_byte_size_exists() {
            let mut store = open();
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
            let mut store = open();
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
            let mut store = open();
            let first_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let second_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_msg_in_a_group_when_exceeding_store_max_byte_size() {
            let mut store = open();
            store.max_byte_size = Some(10);
            let first_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let second_uuid = store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not second msg");
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_lowest_pri_msg_in_the_store_when_exceeding_store_max_byte_size() {
            let mut store = open();
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
            let mut store = open();
            store.max_byte_size = Some(9);
            let result = store.add(&Packet::new(2, "1234567890".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_to_large_for_group_err() {
            let mut store = open();
            store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let result = store.add(&Packet::new(1, "12345678901".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_lacks_priority_err() {
            let mut store = open();
            store.max_byte_size = Some(20);
            store.add(&Packet::new(2, "1234567890".to_string())).expect("Could not add first msg");
            store.add(&Packet::new(2, "1234567890".to_string())).expect("Could not second msg");
            let result = store.add(&Packet::new(1, "1234567890".to_string()));
            assert!(result.is_err());
        }

        #[test]
        fn should_create_group_with_defaults() {
            let mut store = open();
            store.group_defaults.insert(1, GroupDefaults { max_byte_size: Some(10) });
            store.add(&Packet::new(1, "1234567890".to_string())).expect("Could not add msg");
            let group = store.groups_map.get(&1).expect("Could not get group");
            assert_eq!(Some(10), group.max_byte_size);
        }

        #[test]
        fn should_increase_burned_msg_count() {
            let mut store = open();
            store.max_byte_size = Some(3);
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert msg");
            assert_eq!(0, store.msgs_burned);
            store.add(&Packet::new(1, "bar".to_string())).expect("Could not insert msg");
            assert_eq!(1, store.msgs_burned);
            store.max_byte_size = None;
            let mut group = store.groups_map.get_mut(&1).expect("Could not find group");
            group.max_byte_size = Some(3);
            store.add(&Packet::new(1, "baz".to_string())).expect("Could not insert msg");
            assert_eq!(2, store.msgs_burned);
        }

        #[test]
        fn should_reinsert_group_after_errors() {
            let mut store = open();
            store.max_byte_size = Some(10);
            store.add(&Packet::new(2, "12345".to_string())).expect("Could not add msg");
            let first_attempt = store.add(&Packet::new(2, "12345678901".to_string()));
            assert!(first_attempt.is_err());
            let mut group = store.groups_map.get_mut(&2).expect("Group not found");
            group.max_byte_size = Some(5);
            let second_attempt = store.add(&Packet::new(2, "123456".to_string()));
            assert!(second_attempt.is_err());
            let third_attempt = store.add(&Packet::new(1, "123456".to_string()));
            assert!(third_attempt.is_err());
            let group = store.groups_map.get(&2);
            assert!(group.is_some());
        }

    }

    mod get {
        use crate::{
            mem::open,
            store::Packet
        };

        #[test]
        fn should_return_msg() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(Some(uuid), None).expect("Msg not found");
            assert_eq!(uuid, stored_packet.uuid);
            assert_eq!("first message", stored_packet.msg);
        }

        #[test]
        fn should_return_oldest_msg() {
            let mut store = open();
            let first_uuid = store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            store.add(&Packet::new(1, "second message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(None, None).expect("Msg not found");
            assert_eq!(first_uuid, stored_packet.uuid);
            assert_eq!("first message", stored_packet.msg);
        }

        #[test]
        fn should_return_highest_pri_msg() {
            let mut store = open();
            store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            let second_msg = store.add(&Packet::new(2, "second message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(None, None).expect("Msg not found");
            assert_eq!(second_msg, stored_packet.uuid);
            assert_eq!("second message", stored_packet.msg);
        }

        #[test]
        fn should_return_oldest_msg_in_group() {
            let mut store = open();
            let first_uuid = store.add(&Packet::new(1, "first message".to_string())).expect("Could not add first message");
            let _second_uuid = store.add(&Packet::new(2, "second message".to_string())).expect("Could not add first message");
            let _third_uuid = store.add(&Packet::new(1, "third message".to_string())).expect("Could not add first message");
            let stored_packet = store.get(None, Some(1)).expect("Msg not found");
            assert_eq!(first_uuid, stored_packet.uuid);
            assert_eq!("first message", stored_packet.msg);
        }

    }

    mod del {
        use crate::{
            mem::open,
            store::Packet
        };

        #[test]
        fn should_decrease_byte_size() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert first msg");
            store.add(&Packet::new(1, "bar".to_string())).expect("Could not insert second msg");
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(6, store.byte_size);
            assert_eq!(6, group.byte_size);
            assert!(store.db.msgs.get(&uuid).is_some());
            store.del(&uuid).expect("Could not delete message");
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
            assert!(store.db.msgs.get(&uuid).is_none());
        }

        #[test]
        fn should_remove_empty_group() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert first msg");
            assert!(store.groups_map.get(&1).is_some());
            store.del(&uuid).expect("Could not delete message");
            assert!(store.groups_map.get(&1).is_none())
        }

        #[test]
        fn should_increase_bytes_deleted_count() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert first msg");
            assert_eq!(0, store.msgs_deleted);
            store.del(&uuid).expect("Could not delete message");
            assert_eq!(1, store.msgs_deleted);
        }

        #[test]
        fn should_reset_bytes_deleted_count_and_add_diff() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert first msg");
            store.msgs_deleted = i32::MAX;
            assert_eq!(i32::MAX, store.msgs_deleted);
            store.del(&uuid).expect("Could not delete message");
            assert_eq!(1, store.msgs_deleted);
        }

        #[test]
        fn should_clear_deleted_count() {
            let mut store = open();
            let uuid = store.add(&Packet::new(1, "foo".to_string())).expect("Could not insert first msg");
            assert_eq!(0, store.msgs_deleted);
            store.del(&uuid).expect("Could not delete message");
            store.clear_msgs_deleted_count();
            assert_eq!(0, store.msgs_deleted);
        }

    }

    mod update_group_defaults {
        use crate::{
            mem::open,
            store::{
                GroupDefaults,
                Packet
            }
        };

        #[test]
        fn should_update_store_config() {
            let mut store = open();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) });
            let defaults = store.group_defaults.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), defaults.max_byte_size);
        }

        #[test]
        fn should_update_existing_group() {
            let mut store = open();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) });
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add message");
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), group.max_byte_size);
        }

        #[test]
        fn should_prune_group_after_update() {
            let mut store = open();
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add message");
            store.add(&Packet::new(1, "bar".to_string())).expect("Could not add message");
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(3) });            
            let group = store.groups_map.get(&1).expect("Could not find group");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
            assert_eq!(1, store.msgs_burned);
        }

    }

    mod delete_group_defaults {
        use crate::{
            mem::open,
            store::{
                GroupDefaults,
                Packet
            }
        };

        #[test]
        #[test]
        fn should_update_existing_group() {
            let mut store = open();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) });
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add message");
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), group.max_byte_size);
            store.delete_group_defaults(1);
            assert!(store.group_defaults.get(&1).is_none());
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(None, group.max_byte_size);
        }
    }

    mod update_store_defaults {
        use crate::{
            mem::open,
            store::{
                StoreDefaults,
                Packet
            }
        };

        #[test]
        fn should_update_store_config() {
            let mut store = open();
            store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(10) });
            assert_eq!(Some(10), store.max_byte_size);
        }

        #[test]
        fn should_prune_store_after_update() {
            let mut store = open();
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add message");
            store.add(&Packet::new(1, "bar".to_string())).expect("Could not add message");
            store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(3) });            
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
            assert_eq!(1, store.msgs_burned);
        }

    }

    mod clear_counts {
        use crate::{
            mem::open,
            store::{
                Packet
            }
        };

        #[test]
        fn should_clear_msgs_burned_count() {
            let mut store = open();
            store.max_byte_size = Some(3);
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add msg");
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add msg");
            assert_eq!(1, store.msgs_burned);
            store.clear_msgs_burned_count();
            assert_eq!(0, store.msgs_burned);
        }

        #[test]
        fn should_clear_msgs_inserted_count() {
            let mut store = open();
            assert_eq!(0, store.msgs_inserted);
            store.add(&Packet::new(1, "foo".to_string())).expect("Could not add msg");
            assert_eq!(1, store.msgs_inserted);
            store.clear_msgs_inserted_count();
            assert_eq!(0, store.msgs_inserted);
        }   

    }

}