use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::time;

// ID Start
#[derive(Debug, Eq, Clone, Copy)]
pub struct ID {
    pub priority: u64,
    pub timestamp: u128,
    pub sequence: u64,
}

impl Ord for ID {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority > other.priority {
            Ordering::Greater
        } else if self.priority < other.priority {
            Ordering::Less
        } else if self.timestamp < other.timestamp {
            Ordering::Greater
        } else if self.timestamp > other.timestamp {
            Ordering::Less
        } else if self.sequence < other.sequence {
            Ordering::Greater
        } else if self.sequence > other.sequence {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialOrd for ID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.priority > other.priority {
            Some(Ordering::Greater)
        } else if self.priority < other.priority {
            Some(Ordering::Less)
        } else if self.timestamp < other.timestamp {
            Some(Ordering::Greater)
        } else if self.timestamp > other.timestamp {
            Some(Ordering::Less)
        } else if self.sequence < other.sequence {
            Some(Ordering::Greater)
        } else if self.sequence > other.sequence {
            Some(Ordering::Less)
        } else {
            Some(Ordering::Equal)
        }
    }
}

impl PartialEq for ID {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.timestamp == other.timestamp
            && self.sequence == other.sequence
    }
}

pub fn convert_id_to_string(id: &ID) -> String {
    format!("{}-{}-{}", id.priority, id.timestamp, id.sequence)
}

pub fn convert_string_to_id(text: &str) -> ID {
    let values: Vec<&str> = text.split('-').collect();
    let priority: u64 = values[0].parse().unwrap_or_else(|_| panic!("Convert string to id error: Could not convert {} to priority.",
        values[0]));
    let timestamp: u128 = values[1].parse().unwrap_or_else(|_| panic!("Convert string to id error: Could not convert {} to timestamp.",
        values[0]));
    let sequence: u64 = values[2].parse().unwrap_or_else(|_| panic!("Convert string to id error: Could not convert {} to index.",
        values[0]));
    ID {
        priority,
        timestamp,
        sequence,
    }
}

fn generate_id(store: &mut Store, msg: &Msg) -> ID {
    let new_timestamp = generate_timestamp();
    if new_timestamp > store.timestamp {
        store.timestamp = new_timestamp;
        store.timestamp_sequence = 0;
    } else {
        store.timestamp_sequence += 1;
    }
    ID {
        priority: msg.priority,
        timestamp: store.timestamp,
        sequence: store.timestamp_sequence,
    }
}
// ID Finish
pub struct ImportData {
    pub id: Option<ID>,
    pub priority: Option<u64>,
    pub byte_size: Option<u64>,
    pub msg: Option<Vec<u8>>,
}
pub struct Msg {
    pub priority: u64,
    pub byte_size: u64,
}
pub struct InsertResult {
    pub id: ID,
    pub ids_removed: Vec<ID>,
}
pub struct PruningInfo {
    pub store_difference: Option<u64>,
    pub group_difference: Option<u64>,
}
pub struct GroupDefaults {
    pub max_byte_size: Option<u64>,
}
pub struct WorkingGroup {
    pub priority: u64,
    pub group: Group,
}
pub struct Group {
    pub max_byte_size: Option<u64>,
    pub byte_size: u64,
    pub msgs_map: BTreeMap<ID, u64>,
}
pub struct Store {
    pub max_byte_size: Option<u64>,
    pub byte_size: u64,
    pub timestamp: u128,
    pub timestamp_sequence: u64,
    pub group_defaults: BTreeMap<u64, GroupDefaults>,
    pub groups_map: BTreeMap<u64, Group>,
    pub working_group: BTreeMap<u8, WorkingGroup>,
    pub msgs_removed: BTreeMap<u8, Vec<ID>>,
}

pub fn generate_timestamp() -> u128 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn generate_store() -> Store {
    Store {
        max_byte_size: None,
        byte_size: 0,
        timestamp: generate_timestamp(),
        timestamp_sequence: 0,
        group_defaults: BTreeMap::new(),
        groups_map: BTreeMap::new(),
        working_group: BTreeMap::new(),
        msgs_removed: BTreeMap::new(),
    }
}

fn set_working_group(store: &mut Store, priority: &u64) {
    if let Some(group) = store.groups_map.remove(priority) {
        store.working_group.insert(
            0,
            WorkingGroup {
                priority: *priority,
                group,
            },
        );
    } else if let Some(defaults) = store.group_defaults.get(priority) {
        store.working_group.insert(
            0,
            WorkingGroup {
                priority: *priority,
                group: Group {
                    max_byte_size: defaults.max_byte_size,
                    byte_size: 0,
                    msgs_map: BTreeMap::new(),
                },
            },
        );
    } else {
        store.working_group.insert(
            0,
            WorkingGroup {
                priority: *priority,
                group: Group {
                    max_byte_size: None,
                    byte_size: 0,
                    msgs_map: BTreeMap::new(),
                },
            },
        );
    }
}

fn update_working_group(store: &mut Store) {
    let working_group = store.working_group.remove(&0).unwrap();
    store
        .groups_map
        .insert(working_group.priority, working_group.group);
}

fn reject_if_msg_is_too_large_for_store(store: &Store, msg: &Msg) -> Result<(), String> {
    if let Some(store_max_byte_size) = store.max_byte_size {
        if msg.byte_size > store_max_byte_size {
            return Err("Message is too large for store.".to_string());
        }
    }
    Ok(())
}

fn reject_if_msg_is_too_large_for_group(max_byte_size: &u64, msg: &Msg) -> Result<(), String> {
    if &msg.byte_size > max_byte_size {
        return Err("Message is too large for priority group.".to_string());
    }
    Ok(())
}

fn get_bytes_to_remove(max_byte_size: &u64, byte_size: &u64, msg_byte_size: &u64) -> u64 {
    if byte_size <= msg_byte_size {
        *msg_byte_size
    } else {
        (byte_size + msg_byte_size) - max_byte_size
    }
}

fn should_prune(max_byte_size: &u64, new_byte_size: &u64) -> bool {
    max_byte_size < new_byte_size
}

fn indentify_msgs_to_remove_from_group(
    msgs_map: &BTreeMap<ID, u64>,
    bytes_to_remove_from_group: &u64,
    bytes_removed_from_group: &mut u64,
    msgs_removed: &mut Vec<ID>,
) {
    for (id, msg_byte_size) in msgs_map.iter().rev() {
        if *bytes_removed_from_group >= *bytes_to_remove_from_group {
            break;
        }
        msgs_removed.push(*id);
        *bytes_removed_from_group += msg_byte_size;
    }
}

fn remove_designated_ids_from_group(
    store: &mut Store,
    group: &mut Group,
    bytes_removed_from_group: &u64,
    msgs_removed: &Vec<ID>,
) {
    for id in msgs_removed.iter() {
        group.msgs_map.remove(id);
    }
    store.byte_size -= bytes_removed_from_group;
    group.byte_size -= bytes_removed_from_group;
}

fn prune_working_group_if_needed(
    store: &mut Store,
    working_group: &mut WorkingGroup,
    max_byte_size: &u64,
    msgs_removed: &mut Vec<ID>,
    msg: &Msg,
) {
    if should_prune(
        max_byte_size,
        &(working_group.group.byte_size + msg.byte_size),
    ) {
        let bytes_to_remove_from_group = get_bytes_to_remove(
            max_byte_size,
            &working_group.group.byte_size,
            &msg.byte_size,
        );
        let mut bytes_removed_from_group = 0;
        indentify_msgs_to_remove_from_group(
            &working_group.group.msgs_map,
            &bytes_to_remove_from_group,
            &mut bytes_removed_from_group,
            msgs_removed,
        );
        remove_designated_ids_from_group(
            store,
            &mut working_group.group,
            &bytes_removed_from_group,
            msgs_removed,
        );
    }
}

fn prune_working_group(
    store: &mut Store,
    working_group: &mut WorkingGroup,
    msg: &Msg,
    msgs_removed: &mut Vec<ID>,
) -> Result<(), String> {
    if let Some(max_byte_size) = working_group.group.max_byte_size {
        reject_if_msg_is_too_large_for_store(store, msg)?;
        reject_if_msg_is_too_large_for_group(&max_byte_size, msg)?;
        prune_working_group_if_needed(store, working_group, &max_byte_size, msgs_removed, msg);
    }
    Ok(())
}

fn get_bytes_located_in_lower_priority_groups(
    store: &Store,
    bytes_to_remove_from_store: &u64,
    msg: &Msg,
) -> u64 {
    let mut bytes_from_lower_priority_groups = 0;
    for (priority, group) in store.groups_map.iter() {
        if priority >= &msg.priority
            || &bytes_from_lower_priority_groups >= bytes_to_remove_from_store
        {
            break;
        }
        bytes_from_lower_priority_groups += group.byte_size;
    }
    bytes_from_lower_priority_groups
}

fn remove_msgs_and_groups(
    store: &mut Store,
    bytes_to_remove_from_store: &u64,
    bytes_removed_from_store: &mut u64,
    msgs_removed: &mut Vec<ID>,
) {
    let mut groups_removed: Vec<u64> = vec![];
    for (priority, group) in store.groups_map.iter_mut() {
        let mut ids_removed_from_group: Vec<ID> = vec![];
        let mut bytes_removed_from_group: u64 = 0;
        let mut remove_group = true;
        for (id, msg_byte_size) in group.msgs_map.iter().rev() {
            if *bytes_removed_from_store >= *bytes_to_remove_from_store {
                remove_group = false;
                break;
            }
            ids_removed_from_group.push(*id);
            bytes_removed_from_group += msg_byte_size;
            *bytes_removed_from_store += msg_byte_size;
        }
        if remove_group {
            groups_removed.push(*priority);
        } else {
            for id in ids_removed_from_group.iter() {
                group.msgs_map.remove(id);
            }
            group.byte_size -= bytes_removed_from_group;
        }
        msgs_removed.append(&mut ids_removed_from_group);
    }
    for priority in groups_removed.iter() {
        store.groups_map.remove(priority);
    }
}

fn msg_is_lacking_priority(
    bytes_from_lower_priority_groups: &u64,
    group_byte_size: &u64,
    bytes_to_remove_from_store: &u64,
) -> bool {
    &(bytes_from_lower_priority_groups + group_byte_size) < bytes_to_remove_from_store
}

fn reject_if_lacking_priority(
    working_group: &WorkingGroup,
    bytes_from_lower_priority_groups: &u64,
    bytes_to_remove_from_store: &u64,
) -> Result<(), String> {
    let is_lacking_priority = msg_is_lacking_priority(
        bytes_from_lower_priority_groups,
        &working_group.group.byte_size,
        bytes_to_remove_from_store,
    );
    if is_lacking_priority {
        return Err("The message lacks priority.".to_string());
    }
    Ok(())
}

fn prune_groups(
    store: &mut Store,
    msg: &Msg,
    working_group: &mut WorkingGroup,
    msgs_removed: &mut Vec<ID>,
) -> Result<(), String> {
    if let Some(max_byte_size) = store.max_byte_size {
        if msg.byte_size > max_byte_size {
            return Err("Message is too large for store.".to_string());
        }
        if max_byte_size < (store.byte_size + msg.byte_size) {
            let bytes_to_remove_from_store =
                get_bytes_to_remove(&max_byte_size, &store.byte_size, &msg.byte_size);
            let bytes_from_lower_priority_groups =
                get_bytes_located_in_lower_priority_groups(store, &bytes_to_remove_from_store, msg);
            reject_if_lacking_priority(
                working_group,
                &bytes_from_lower_priority_groups,
                &bytes_to_remove_from_store,
            )?;
            let mut bytes_removed_from_store = 0;
            remove_msgs_and_groups(
                store,
                &bytes_to_remove_from_store,
                &mut bytes_removed_from_store,
                msgs_removed,
            );
            prune_working_group_if_needed(store, working_group, &max_byte_size, msgs_removed, msg);
            store.byte_size -= bytes_removed_from_store;
        }
    }
    Ok(())
}

// Store Actions Start
fn prepare_store(store: &mut Store, msg: &Msg) -> Result<(), String> {
    let mut working_group = { store.working_group.remove(&0).unwrap() };
    let mut msgs_removed: Vec<ID> = vec![];
    prune_working_group(store, &mut working_group, msg, &mut msgs_removed)?;
    prune_groups(store, msg, &mut working_group, &mut msgs_removed)?;
    store.msgs_removed.insert(0, msgs_removed);
    store.working_group.insert(0, working_group);
    Ok(())
}

fn insert_msg_in_working_group(store: &mut Store, msg_byte_size: &u64, id: &ID) {
    let working_group = store.working_group.get_mut(&0).unwrap();
    store.byte_size += msg_byte_size;
    working_group.group.byte_size += msg_byte_size;
    working_group.group.msgs_map.insert(*id, *msg_byte_size);
}

fn generate_insert_result(store: &mut Store, id: ID) -> InsertResult {
    
    InsertResult {
        id,
        ids_removed: store.msgs_removed.remove(&0).unwrap(),
    }
}

pub fn insert(store: &mut Store, msg: &Msg) -> Result<InsertResult, String> {
    set_working_group(store, &msg.priority);
    prepare_store(store, msg)?;
    let id = generate_id(store, msg);
    insert_msg_in_working_group(store, &msg.byte_size, &id);
    update_working_group(store);
    let result = generate_insert_result(store, id);
    Ok(result)
}

pub fn delete(store: &mut Store, id: &ID) -> Result<bool, String> {
    let mut remove_group = false;
    let mut msg_removed = false;
    if let Some(group) = store.groups_map.get_mut(&id.priority) {
        if let Some(msg_byte_size) = group.msgs_map.remove(id) {
            msg_removed = true;
            group.byte_size -= msg_byte_size;
            store.byte_size -= msg_byte_size;
            if group.msgs_map.is_empty() {
                remove_group = true;
            }
        }
    }
    if remove_group {
        store.groups_map.remove(&id.priority);
    }
    Ok(msg_removed)
}

pub fn get_next(store: &mut Store) -> Result<Option<ID>, String> {
    if let Some((_priority, group)) = store.groups_map.iter().rev().next() {
        return Ok(group.msgs_map.keys().rev().next().cloned());
    } else {
        Ok(None)
    }
}

// Store Actions Finish

#[cfg(test)]
mod tests {

    use std::cmp::Ordering;

    use crate::store::{
        convert_id_to_string, convert_string_to_id, delete, generate_id, generate_store, get_next,
        insert, GroupDefaults, Msg, ID,
    };

    #[test]
    fn should_insert_two_msgs_and_return_the_oldest_highest_msg() {
        let mut store = generate_store();
        let mut msg = Msg {
            priority: 1,
            byte_size: "0123456789".len() as u64,
        };
        // insert one document
        let msg_1 = insert(&mut store, &msg).unwrap();
        assert_eq!(store.byte_size, 10);
        assert_eq!(store.groups_map.len(), 1);
        let group = store
            .groups_map
            .get(&msg.priority)
            .expect("Collection not found");
        assert_eq!(group.byte_size, 10);
        assert_eq!(group.msgs_map.len(), 1);
        // insert second document to same group
        insert(&mut store, &msg).unwrap();
        // get_next should get the oldest of the two
        let returned_id = get_next(&mut store).unwrap().unwrap();
        assert_eq!(msg_1.id, returned_id);
        // insert a lower priority msg and then get_next should still return the older-higher priority msg
        msg.priority = 0;
        insert(&mut store, &msg).unwrap();
        let returned_id = get_next(&mut store).unwrap().unwrap();
        assert_eq!(msg_1.id, returned_id);
    }

    #[test]
    fn should_reject_messages_that_exceed_max_sizes() {
        let mut store = generate_store();
        store.max_byte_size = Some(99);
        let mut msg = Msg {
            priority: 1,
            byte_size: 100,
        };
        assert_eq!(true, insert(&mut store, &msg).is_err());
        store.group_defaults.insert(
            1,
            GroupDefaults {
                max_byte_size: Some(99),
            },
        );
        assert_eq!(true, insert(&mut store, &msg).is_err());
        msg.byte_size = 99;
        assert_eq!(true, insert(&mut store, &msg).is_ok());
        msg.priority = 0;
        assert_eq!(true, insert(&mut store, &msg).is_err());
        {
            let group = store.groups_map.get_mut(&1).unwrap();
            group.max_byte_size = Some(9);
        }
        msg.priority = 1;
        msg.byte_size = 10;
        assert_eq!(true, insert(&mut store, &msg).is_err());
    }

    #[test]
    fn should_delete_lower_priority_msgs() {
        let mut store = generate_store();
        store.max_byte_size = Some(10);
        let mut msg = Msg {
            priority: 0,
            byte_size: 10,
        };
        let result_1 = insert(&mut store, &msg).unwrap();
        let result_2 = insert(&mut store, &msg).unwrap();
        {
            let group = store.groups_map.get(&0).unwrap();
            assert_eq!(result_1.id, result_2.ids_removed[0]);
            assert_eq!(10, store.byte_size);
            assert_eq!(10, group.byte_size);
            assert_eq!(1, group.msgs_map.len());
        }
        msg.priority = 1;
        let result_3 = insert(&mut store, &msg).unwrap();
        let group_0 = store.groups_map.get(&0);
        let group_1 = store.groups_map.get(&1).unwrap();
        assert_eq!(result_2.id, result_3.ids_removed[0]);
        assert_eq!(10, store.byte_size);
        assert_eq!(10, group_1.byte_size);
        assert_eq!(1, group_1.msgs_map.len());
        assert_eq!(true, group_0.is_none());
        assert_eq!(1, store.groups_map.len());
    }

    #[test]
    fn should_removed_msgs_from_store() {
        let mut store = generate_store();
        let msg = Msg {
            priority: 0,
            byte_size: 10,
        };
        let insert_1 = insert(&mut store, &msg).unwrap();
        let insert_2 = insert(&mut store, &msg).unwrap();
        assert_eq!(true, delete(&mut store, &insert_1.id).unwrap());
        assert_eq!(true, delete(&mut store, &insert_2.id).unwrap());
        assert_eq!(false, delete(&mut store, &insert_2.id).unwrap());
        assert_eq!(0, store.byte_size);
        assert_eq!(0, store.groups_map.len());
    }

    #[test]
    fn should_format_between_id_and_string() {
        let id = ID {
            priority: 0,
            timestamp: 0,
            sequence: 0,
        };
        let text = convert_id_to_string(&id);
        let from_text = convert_string_to_id(&text);
        assert_eq!("0-0-0".to_string(), text);
        assert_eq!(id, from_text);
    }

    #[test]
    fn ids_should_be_sorted_by_pri_then_timestamp() {
        let id_1 = ID {
            priority: 1,
            timestamp: 10,
            sequence: 10,
        };
        let id_2 = ID {
            priority: 2,
            timestamp: 10,
            sequence: 10,
        };
        let id_3 = ID {
            priority: 2,
            timestamp: 9,
            sequence: 1,
        };
        let id_4 = ID {
            priority: 2,
            timestamp: 9,
            sequence: 0,
        };
        let id_5 = ID {
            priority: 2,
            timestamp: 9,
            sequence: 0,
        };
        assert_eq!(Ordering::Greater, id_2.cmp(&id_1));
        assert_eq!(Ordering::Greater, id_3.cmp(&id_2));
        assert_eq!(Ordering::Greater, id_4.cmp(&id_3));
        assert_eq!(Ordering::Less, id_1.cmp(&id_2));
        assert_eq!(Ordering::Less, id_2.cmp(&id_3));
        assert_eq!(Ordering::Less, id_3.cmp(&id_4));
        assert_eq!(Ordering::Equal, id_4.cmp(&id_5));
        assert_eq!(Some(Ordering::Greater), id_2.partial_cmp(&id_1));
        assert_eq!(Some(Ordering::Greater), id_3.partial_cmp(&id_2));
        assert_eq!(Some(Ordering::Greater), id_4.partial_cmp(&id_3));
        assert_eq!(Some(Ordering::Less), id_1.partial_cmp(&id_2));
        assert_eq!(Some(Ordering::Less), id_2.partial_cmp(&id_3));
        assert_eq!(Some(Ordering::Less), id_3.partial_cmp(&id_4));
        assert_eq!(Some(Ordering::Equal), id_4.partial_cmp(&id_5));
    }

    #[test]
    pub fn timesamp_sequence_should_reset_with_each_timestamp() {
        let mut store = generate_store();
        store.timestamp = u128::MIN;
        store.timestamp_sequence = 10;
        generate_id(
            &mut store,
            &Msg {
                priority: 0,
                byte_size: 1,
            },
        );
        let timestamp_1 = store.timestamp;
        let sequence_1 = store.timestamp_sequence;
        store.timestamp = u128::MAX;
        store.timestamp_sequence = 0;
        generate_id(
            &mut store,
            &Msg {
                priority: 0,
                byte_size: 1,
            },
        );
        let timestamp_2 = store.timestamp;
        let sequence_2 = store.timestamp_sequence;
        assert_eq!(true, timestamp_1 > u128::MIN);
        assert_eq!(0, sequence_1);
        assert_eq!(true, timestamp_2 == u128::MAX);
        assert_eq!(1, sequence_2);
    }

    #[test]
    fn it_should_removed_bytes_from_working_group() {
        let mut store = generate_store();
        store.group_defaults.insert(
            0,
            GroupDefaults {
                max_byte_size: Some(10),
            },
        );
        let mut ids_inserted: Vec<ID> = vec![];
        for _ in 1..=5 {
            let result = insert(
                &mut store,
                &Msg {
                    priority: 0,
                    byte_size: 2,
                },
            )
            .unwrap();
            ids_inserted.push(result.id);
        }
        let result_1 = insert(
            &mut store,
            &Msg {
                priority: 0,
                byte_size: 4,
            },
        )
        .unwrap();
        assert_eq!(ids_inserted[0], result_1.ids_removed[0]);
        assert_eq!(ids_inserted[1], result_1.ids_removed[1]);
        store.group_defaults.insert(
            1,
            GroupDefaults {
                max_byte_size: Some(10),
            },
        );
        let result_2 = insert(
            &mut store,
            &Msg {
                priority: 1,
                byte_size: 5,
            },
        )
        .unwrap();
        let result_3 = insert(
            &mut store,
            &Msg {
                priority: 1,
                byte_size: 6,
            },
        )
        .unwrap();
        assert_eq!(result_2.id, result_3.ids_removed[0]);
    }

    #[test]
    fn it_should_not_remove_group_but_msgs_from_group() {
        let mut store = generate_store();
        store.max_byte_size = Some(10);
        let _result_1 = insert(
            &mut store,
            &Msg {
                priority: 1,
                byte_size: 5,
            },
        )
        .unwrap();
        let _result_2 = insert(
            &mut store,
            &Msg {
                priority: 1,
                byte_size: 5,
            },
        )
        .unwrap();
        let _result_3 = insert(
            &mut store,
            &Msg {
                priority: 2,
                byte_size: 5,
            },
        )
        .unwrap();
        let group_1 = store.groups_map.get(&1).unwrap();
        let group_2 = store.groups_map.get(&2).unwrap();
        assert_eq!(1, group_1.msgs_map.len());
        assert_eq!(5, group_1.byte_size);
        assert_eq!(10, store.byte_size);
        assert_eq!(5, group_2.byte_size);
        assert_eq!(1, group_2.msgs_map.len());
    }

    #[test]
    fn it_should_return_ok_none_when_no_group_is_present() {
        let mut store = generate_store();
        assert_eq!(None, get_next(&mut store).unwrap());
    }

    #[test]
    fn it_should_return_ok_false_when_deleting_a_msg_that_does_not_exist() {
        let mut store = generate_store();
        insert(&mut store, &Msg { priority: 0, byte_size: 10 }).unwrap();
        let result_1 = delete(&mut store, &ID { priority: 0, timestamp: 0, sequence: 0 }).unwrap();
        assert!(!result_1)
    }
}
