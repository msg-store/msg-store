use crate::{
    store::Store,
    uuid::Uuid
};
use std::collections::BTreeMap;

/// This is a store which utilizes an in memory database.
/// 
/// The database is simply a BTreeMap.
/// 
/// This Store also comes with an associated function called "open".

pub struct MemDb {
    msgs: BTreeMap<Uuid, String>
}

impl MemDb {
    pub fn new() -> MemDb {
        MemDb {
            msgs: BTreeMap::new()
        }
    }
    pub fn add(&mut self, uuid: Uuid, msg: String) {
        self.msgs.insert(uuid, msg);
    }
    pub fn get(&mut self, uuid: &Uuid) -> Option<String> {
        match self.msgs.get(uuid) {
            Some(msg) => Some(msg.clone()),
            None => None
        }
    }
    pub fn del(&mut self, uuid: &Uuid) {
        self.msgs.remove(uuid);
    }
}

pub fn open() -> (Store, MemDb) {
    (Store::new(), MemDb::new())
}

#[cfg(test)]
mod tests {

    mod add {
        use crate::store::{ Store, GroupDefaults };

        #[test]
        fn should_increase_store_byte_size() {
            let mut store = Store::new();
            store.add(1, "1234567890").expect("Could not add msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_increase_group_byte_size() {
            let mut store = Store::new();
            store.add(1, "1234567890").expect("Could not add msg");
            let group = store.groups_map.get(&1).expect("Could not find group");
            assert_eq!(group.byte_size, 10)
        }

        #[test]
        fn should_prune_store_byte_size_to_10_when_store_max_byte_size_exists() {
            let mut store = Store::new();
            store.max_byte_size = Some(10);
            store.add(1, "1234567890").expect("Could not add first msg");
            store.add(1, "1234567890").expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_prune_store_byte_size_to_10_when_group_max_byte_size_exists() {
            let mut store = Store::new();
            store.add(1, "1234567890").expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not find group");
            group.max_byte_size = Some(10);
            store.add(1, "1234567890").expect("Could not second msg");
            assert_eq!(store.byte_size, 10)
        }

        #[test]
        fn should_prune_group_byte_size_to_10_when_group_max_byte_size_exists() {
            let mut store = Store::new();
            store.add(1, "1234567890").expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            store.add(1, "1234567890").expect("Could not second msg");
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(group.byte_size, 10)
        }

        #[test]
        fn should_prune_oldest_msg_in_a_group_when_exceeding_group_max_byte_size() {
            let mut store = Store::new();
            let first_uuid = store.add(1, "1234567890").expect("Could not add first msg").uuid;
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let second_uuid = store.add(1, "1234567890").expect("Could not second msg").uuid;
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_msg_in_a_group_when_exceeding_store_max_byte_size() {
            let mut store = Store::new();
            store.max_byte_size = Some(10);
            let first_uuid = store.add(1, "1234567890").expect("Could not add first msg").uuid;
            let second_uuid = store.add(1, "1234567890").expect("Could not second msg").uuid;
            assert_eq!(None, store.id_to_group_map.get(&first_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&second_uuid));
        }

        #[test]
        fn should_prune_oldest_lowest_pri_msg_in_the_store_when_exceeding_store_max_byte_size() {
            let mut store = Store::new();
            store.max_byte_size = Some(20);
            let first_uuid = store.add(2, "1234567890").expect("Could not add first msg").uuid;
            let second_uuid = store.add(1, "1234567890").expect("Could not second msg").uuid;
            let third_uuid = store.add(1, "1234567890").expect("Could not second msg").uuid;
            assert_eq!(Some(&2), store.id_to_group_map.get(&first_uuid));
            assert_eq!(None, store.id_to_group_map.get(&second_uuid));
            assert_eq!(Some(&1), store.id_to_group_map.get(&third_uuid));
        }

        #[test]
        fn should_return_msg_to_large_for_store_err() {
            let mut store = Store::new();
            store.max_byte_size = Some(9);
            let result = store.add(2, "1234567890");
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_to_large_for_group_err() {
            let mut store = Store::new();
            store.add(1, "1234567890").expect("Could not add first msg");
            let mut group = store.groups_map.get_mut(&1).expect("Could not get mutable group");
            group.max_byte_size = Some(10);
            let result = store.add(1, "12345678901");
            assert!(result.is_err());
        }

        #[test]
        fn should_return_msg_lacks_priority_err() {
            let mut store = Store::new();
            store.max_byte_size = Some(20);
            store.add(2, "1234567890").expect("Could not add first msg");
            store.add(2, "1234567890").expect("Could not second msg");
            let result = store.add(1, "1234567890");
            assert!(result.is_err());
        }

        #[test]
        fn should_create_group_with_defaults() {
            let mut store = Store::new();
            store.group_defaults.insert(1, GroupDefaults { max_byte_size: Some(10) });
            store.add(1, "1234567890").expect("Could not add msg");
            let group = store.groups_map.get(&1).expect("Could not get group");
            assert_eq!(Some(10), group.max_byte_size);
        }

        #[test]
        fn should_reinsert_group_after_errors() {
            let mut store = Store::new();
            store.max_byte_size = Some(10);
            store.add(2, "12345").expect("Could not add msg");
            let first_attempt = store.add(2, "12345678901");
            assert!(first_attempt.is_err());
            let mut group = store.groups_map.get_mut(&2).expect("Group not found");
            group.max_byte_size = Some(5);
            let second_attempt = store.add(2, "123456");
            assert!(second_attempt.is_err());
            let third_attempt = store.add(1, "123456");
            assert!(third_attempt.is_err());
            let group = store.groups_map.get(&2);
            assert!(group.is_some());
        }

    }

    mod get {
        use crate::store::Store;

        #[test]
        fn should_return_msg() {
            let mut store = Store::new();
            let uuid = store.add(1, "first message").unwrap().uuid;
            let stored_packet = store.get(Some(uuid), None, false).unwrap().expect("Msg not found");
            assert_eq!(uuid, stored_packet);
        }

        #[test]
        fn should_return_oldest_msg() {
            let mut store = Store::new();
            let first_uuid = store.add(1, "first message").unwrap().uuid;
            store.add(1, "second message").unwrap();
            let stored_packet = store.get(None, None, false).unwrap().expect("Msg not found");
            assert_eq!(first_uuid, stored_packet);
        }

        #[test]
        fn should_return_youngest_msg() {
            let mut store = Store::new();
            let _first_uuid = store.add(1, "first message").unwrap().uuid;
            let second_uuid = store.add(1, "second message").unwrap().uuid;
            let stored_packet = store.get(None, None, true).unwrap().expect("Msg not found");
            assert_eq!(second_uuid, stored_packet);
        }

        #[test]
        fn should_return_highest_pri_msg() {
            let mut store = Store::new();
            store.add(1, "first message").unwrap();
            let second_msg = store.add(2, "second message").unwrap().uuid;
            let stored_packet = store.get(None, None, false).unwrap().expect("Msg not found");
            assert_eq!(second_msg, stored_packet);
        }

        #[test]
        fn should_return_lowest_pri_msg() {
            let mut store = Store::new();
            let first_msg = store.add(1, "first message").unwrap().uuid;
            let _second_msg = store.add(2, "second message").unwrap().uuid;
            let stored_packet = store.get(None, None, true).unwrap().expect("Msg not found");
            assert_eq!(first_msg, stored_packet);
        }

        #[test]
        fn should_return_oldest_msg_in_group() {
            let mut store = Store::new();
            let first_uuid = store.add(1, "first message").unwrap().uuid;
            let _second_uuid = store.add(2, "second message").unwrap().uuid;
            let _third_uuid = store.add(1, "third message").unwrap().uuid;
            let stored_packet = store.get(None,Some(1), false).unwrap().expect("Msg not found");
            assert_eq!(first_uuid, stored_packet);
        }

        #[test]
        fn should_return_youngest_msg_in_group() {
            let mut store = Store::new();
            let _first_uuid = store.add(1, "first message").unwrap().uuid;
            let _second_uuid = store.add(2, "second message").unwrap().uuid;
            let third_uuid = store.add(1, "third message").unwrap().uuid;
            let stored_packet = store.get(None,Some(1), true).unwrap().expect("Msg not found");
            assert_eq!(third_uuid, stored_packet);
        }

    }

    mod get_metadata {
        use crate::store::Store;

        #[test]
        fn should_return_2_message_data_points() {
            let mut store = Store::new();
            let uuid1 = store.add(1, "first message").unwrap().uuid;
            let uuid2 = store.add(1, "second message").unwrap().uuid;
            let set = store.get_metadata((0, 1), None);
            assert_eq!(2, set.len());
            assert_eq!(uuid1, set[0].uuid);
            assert_eq!(uuid2, set[1].uuid);
        }

        #[test]
        fn should_return_2_message_data_points_with_range_starting_at_2() {
            let mut store = Store::new();
            let _uuid1 = store.add(1, "first message").unwrap().uuid;
            let _uuid2 = store.add(1, "second message").unwrap().uuid;
            let _uuid3 = store.add(1, "third message").unwrap().uuid;
            let set = store.get_metadata((1, 2), None);
            assert_eq!(2, set.len());
        }
    
    }

    mod del {
        use crate::store::Store;

        #[test]
        fn should_decrease_byte_size() {
            let mut store = Store::new();
            let uuid = store.add(1, "foo").unwrap().uuid;
            store.add(1, "bar").unwrap();
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(6, store.byte_size);
            assert_eq!(6, group.byte_size);
            store.del(&uuid).unwrap();
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
        }

        #[test]
        fn should_remove_empty_group() {
            let mut store = Store::new();
            let uuid = store.add(1, "foo").unwrap().uuid;
            assert!(store.groups_map.get(&1).is_some());
            store.del(&uuid).unwrap();
            assert!(store.groups_map.get(&1).is_none())
        }

    }

    mod del_group {
        use crate::store::Store;

        #[test]
        fn should_decrease_byte_size() {
            let mut store = Store::new();
            store.add(1, "foo").unwrap();
            store.add(1, "bar").unwrap();
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(6, store.byte_size);
            assert_eq!(6, group.byte_size);
            store.del_group(&1).unwrap();
            assert_eq!(true, store.groups_map.get(&1).is_none());
            assert_eq!(0, store.byte_size);
        }

        #[test]
        fn should_remove_empty_group() {
            let mut store = Store::new();
            store.add(1, "foo").unwrap();
            store.add(1, "bar").unwrap();
            let group = store.groups_map.get(&1).expect("Could get group ref");
            assert_eq!(6, store.byte_size);
            assert_eq!(6, group.byte_size);
            store.del_group(&1).unwrap();
            assert_eq!(true, store.groups_map.get(&1).is_none());
        }

    }

    mod update_group_defaults {
        use crate::store::{ Store, GroupDefaults };

        #[test]
        fn should_update_store_config() {
            let mut store = Store::new();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) }).unwrap();
            let defaults = store.group_defaults.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), defaults.max_byte_size);
        }

        #[test]
        fn should_update_existing_group() {
            let mut store = Store::new();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) }).unwrap();
            store.add(1, "foo").unwrap();
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), group.max_byte_size);
        }

        #[test]
        fn should_prune_group_after_update() {
            let mut store = Store::new();
            store.add(1, "foo").unwrap();
            store.add(1, "bar").unwrap();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(3) }).unwrap();            
            let group = store.groups_map.get(&1).expect("Could not find group");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
        }

    }

    mod delete_group_defaults {
        use crate::store::{Store, GroupDefaults};

        #[test]
        #[test]
        fn should_update_existing_group() {
            let mut store = Store::new();
            store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(10) }).unwrap();
            store.add(1, "foo").unwrap();
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(Some(10), group.max_byte_size);
            store.delete_group_defaults(1);
            assert!(store.group_defaults.get(&1).is_none());
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(None, group.max_byte_size);
        }
    }

    mod update_store_defaults {
        use crate::store::{ Store, StoreDefaults };

        #[test]
        fn should_update_store_config() {
            let mut store = Store::new();
            store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(10) }).unwrap();
            assert_eq!(Some(10), store.max_byte_size);
        }

        #[test]
        fn should_prune_store_after_update() {
            let mut store = Store::new();
            store.add(1, "foo").unwrap();
            store.add(1, "bar").unwrap();
            store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(3) }).unwrap();            
            let group = store.groups_map.get(&1).expect("Could not find defaults");
            assert_eq!(3, store.byte_size);
            assert_eq!(3, group.byte_size);
        }

    }

    mod uuid {
        use crate::Uuid;

        #[test]
        fn should_convert_a_str_to_uuid() {
            assert_eq!(Uuid{ timestamp: 1636523479865480266, sequence: 1 }, Uuid::from_string("1636523479865480266-1").unwrap())
        }
    }

}