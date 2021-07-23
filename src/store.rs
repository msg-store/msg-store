use std::collections::BTreeMap;
use std::cmp::Ordering;
use std::time;

// ID Start
#[derive(Debug, Eq, Clone, Copy)]
pub struct ID {
    pub priority: u64,
    pub timestamp: u128,
    pub sequence: u64
}

impl Ord for ID {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority > other.priority {
            Ordering::Greater
        } else if self.priority < other.priority {
            Ordering::Less
        } else {
            if self.timestamp < other.timestamp {
                Ordering::Less
            } else if self.timestamp > other.timestamp {
                Ordering::Greater
            } else {
                if self.sequence < other.sequence {
                    Ordering::Greater
                } else if self.sequence > other.sequence {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            }
        }
    }
}

impl PartialOrd for ID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.priority > other.priority {
            Some(Ordering::Greater)
        } else if self.priority < other.priority {
            Some(Ordering::Less)
        } else {
            if self.timestamp < other.timestamp {
                Some(Ordering::Less)
            } else if self.timestamp > other.timestamp {
                Some(Ordering::Greater)
            } else {
                if self.sequence < other.sequence {
                    Some(Ordering::Greater)
                } else if self.sequence > other.sequence {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Equal)
                }
            }
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
    let values: Vec<&str> = text.split("-").collect();
    let priority: u64 = values[0].parse().expect(&format!("Convert string to id error: Could not convert {} to priority.", values[0]));
    let timestamp: u128 = values[1].parse().expect(&format!("Convert string to id error: Could not convert {} to timestamp.", values[0]));
    let sequence: u64 = values[2].parse().expect(&format!("Convert string to id error: Could not convert {} to index.", values[0]));
    ID {
        priority,
        timestamp,
        sequence
    }
}

fn generate_id(store: &mut Store, msg: &Msg) -> ID {
    let new_timestamp = generate_timestamp();
    if new_timestamp > store.timestamp {
        store.timestamp = new_timestamp;
        store.timestamp_sequence = 0;
    } else  {
        store.timestamp_sequence += 1;
    }
    ID {
        priority: msg.priority.clone(),
        timestamp: store.timestamp.clone(),
        sequence: store.timestamp_sequence.clone()
    }
}
// ID Finish
pub struct ImportData {
    pub id: Option<ID>,
    pub priority: Option<u64>,
    pub byte_size: Option<u64>,
    pub msg: Option<Vec<u8>>
}
pub struct Msg {
    pub priority: u64,
    pub byte_size: u64
}
pub struct InsertResult{
    pub id: ID,
    pub ids_removed: Vec<ID>
}
pub struct PruningInfo {
    pub store_difference: Option<u64>,
    pub group_difference: Option<u64>
}
pub struct GroupDefaults {
    pub max_byte_size: Option<u64>
}
pub struct WorkingGroup {
    pub priority: u64,
    pub group: Group
}
pub struct Group {
    pub max_byte_size: Option<u64>,
    pub byte_size: u64,
    pub msgs_map: BTreeMap<ID, u64>
}
pub struct Store {
    pub max_byte_size: Option<u64>,
    pub byte_size: u64,
    pub timestamp: u128,
    pub timestamp_sequence: u64,
    pub group_defaults: BTreeMap<u64, GroupDefaults>,
    pub groups_map: BTreeMap<u64, Group>,
    pub working_group: BTreeMap<u8, WorkingGroup>,
    pub msgs_removed: BTreeMap<u8, Vec<ID>>
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
        msgs_removed: BTreeMap::new()
    }
}

fn set_working_group(store: &mut Store, priority: &u64) {
    if let Some(group) = store.groups_map.remove(priority) {
        store.working_group.insert(0, WorkingGroup {
            priority: *priority,
            group
        });
    } else {
        if let Some(defaults) = store.group_defaults.get(&priority) {
            store.working_group.insert(0, WorkingGroup {
                priority: *priority,
                group: Group {
                    max_byte_size: defaults.max_byte_size,
                    byte_size: 0,
                    msgs_map: BTreeMap::new()
                }
            });
        } else {
            store.working_group.insert(0, WorkingGroup {
                priority: *priority,
                group: Group {
                    max_byte_size: None,
                    byte_size: 0,
                    msgs_map: BTreeMap::new()
                }
            });
        }
    }
}

fn update_working_group(store: &mut Store) {
    let working_group = store.working_group.remove(&0).unwrap();
    store.groups_map.insert(working_group.priority, working_group.group);
}

// Store Actions Start
fn prepare_store(store: &mut Store, msg: &Msg) -> Result<(), String> {
    let working_group = store.working_group.get_mut(&0).unwrap();
    let mut msgs_removed: Vec<ID> = vec![];
    if let Some(max_byte_size) = working_group.group.max_byte_size {
        if let Some(store_max_byte_size) = store.max_byte_size {
            if msg.byte_size > store_max_byte_size {
                return Err(format!("Message is too large for store."));
            }
        }
        if max_byte_size > max_byte_size {
            return Err(format!("Message is too large for priority group."))
        }
        if max_byte_size < working_group.group.byte_size + max_byte_size {
            let bytes_to_remove_from_group = {
                if working_group.group.byte_size <= msg.byte_size {
                    msg.byte_size
                } else {
                    (working_group.group.byte_size + msg.byte_size) + max_byte_size
                }
            };
            let mut bytes_removed_from_group = 0;
            for (id, msg_byte_size) in working_group.group.msgs_map.iter() {
                if bytes_removed_from_group >= bytes_to_remove_from_group {
                    break;
                }
                msgs_removed.push(*id);
                bytes_removed_from_group += msg_byte_size;
            }
            for id in msgs_removed.iter() {
                working_group.group.msgs_map.remove(&id);
            }
            store.byte_size -= bytes_removed_from_group;
            working_group.group.byte_size -= bytes_removed_from_group;
        }
    }
    if let Some(max_byte_size) = store.max_byte_size {
        if msg.byte_size > max_byte_size {
            return Err(format!("Message is too large for store."));
        }
        if max_byte_size < (store.byte_size + max_byte_size) {
            let bytes_to_remove_from_store = {
                if store.byte_size <= msg.byte_size {
                    msg.byte_size
                } else {
                    (store.byte_size + msg.byte_size) + max_byte_size
                }
            };
            let mut bytes_from_lower_priority_groups = 0;
            for (priority, group) in store.groups_map.iter() {
                if priority >= &msg.priority || bytes_from_lower_priority_groups >= bytes_to_remove_from_store {
                    break;
                }
                bytes_from_lower_priority_groups += group.byte_size;
            }
            if (bytes_from_lower_priority_groups + working_group.group.byte_size) < bytes_to_remove_from_store {
                return Err(format!("The message lacks priority."));
            }
            let mut bytes_removed_from_store = 0;
            let mut groups_removed: Vec<u64> = vec![];
            for (priority, group) in store.groups_map.iter_mut() {
                let mut ids_removed_from_group: Vec<ID> = vec![];
                let mut remove_group = false;
                for (id, msg_byte_size) in group.msgs_map.iter() {
                    if priority > &msg.priority {
                        break;
                    }
                    if bytes_removed_from_store >= bytes_to_remove_from_store {
                        remove_group = true;
                        break;
                    }
                    ids_removed_from_group.push(*id);
                    bytes_removed_from_store += msg_byte_size;
                }
                if remove_group {
                    groups_removed.push(*priority);
                } else {
                    for id in ids_removed_from_group.iter() {
                        group.msgs_map.remove(&id);
                    }
                }
                msgs_removed.append(&mut ids_removed_from_group);
            }
            for priority in groups_removed.iter() {
                store.groups_map.remove(&priority);
            }
            let mut msgs_removed_from_group: Vec<ID> = vec![];
            let mut bytes_removed_from_group = 0;
            if bytes_to_remove_from_store > bytes_removed_from_store {
                for (id, msg_byte_size) in working_group.group.msgs_map.iter() {
                    if bytes_removed_from_store >= bytes_to_remove_from_store {
                        break;
                    }
                    msgs_removed_from_group.push(*id);
                    bytes_removed_from_store += msg_byte_size;
                    bytes_removed_from_group += msg_byte_size;
                }
                for id in msgs_removed_from_group.iter() {
                    working_group.group.msgs_map.remove(&id);
                }
                working_group.group.byte_size -= bytes_removed_from_group;
                msgs_removed.append(&mut msgs_removed_from_group);
            }
            store.byte_size -= bytes_removed_from_store;
        }
    }
    store.msgs_removed.insert(0, msgs_removed);
    Ok(())
}

fn insert_msg_in_working_group(store: &mut Store, msg_byte_size: &u64, id: &ID) {
    let working_group = store.working_group.get_mut(&0).unwrap();
    store.byte_size += msg_byte_size;
    working_group.group.byte_size += msg_byte_size;
    working_group.group.msgs_map.insert(*id, *msg_byte_size);
}

fn generate_insert_result(store: &mut Store, id: ID) -> InsertResult {
    let result = InsertResult {
        id,
        ids_removed: store.msgs_removed.remove(&0).unwrap()
    };
    result
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

pub fn delete(store: &mut Store, id: &ID) -> Result<(), String> {
    let mut remove_group = false;
    if let Some(group) = store.groups_map.get_mut(&id.priority) {
       if let Some(msg_byte_size) = group.msgs_map.remove(&id) {
            group.byte_size -= msg_byte_size;
            store.byte_size -= msg_byte_size;
            if group.msgs_map.len() == 0 {
                remove_group = true;
            }
       }
    }
    if remove_group {
        store.groups_map.remove(&id.priority);
    }
    Ok(())
}

pub fn get_next(store: &mut Store) -> Result<Option<ID>, String> {
    let mut next_id: Option<ID> = None;
    for (_, group) in store.groups_map.iter().rev() {
        for (id, _) in group.msgs_map.iter().rev() {
           next_id = Some(*id); 
           break;
        }
        if next_id.is_some() {
            break;
        }
    }
   Ok(next_id) 
}

pub fn import_msg(store: &mut Store, data: &ImportData) -> Result<(), String> {
    let msg_id: ID;
    let msg_priority: u64;
    let msg_byte_size: u64;
    if let Some(data_byte_size) = data.byte_size {
        msg_byte_size = data_byte_size;
    } else {
        if let Some(msg) = &data.msg {
            msg_byte_size = msg.len() as u64;
        } else {
            return Err(format!("The msg byte size could not be determined"));
        }
    }
    if let Some(data_id) = data.id {
        msg_id = data_id;
        msg_priority = msg_id.priority;
    } else {
        if let Some(data_priority) = data.priority {
            msg_priority = data_priority;
        } else {
            return Err(format!("The msg priority could not be determined"));
        }
        let msg = Msg {
            priority: msg_priority,
            byte_size: 0
        };
        msg_id = generate_id(store, &msg);
    }
    let msg = Msg {
        priority: msg_priority,
        byte_size: msg_byte_size
    };
    // TODO: FIX: This may push out msgs that may have higher priority withing the same group
    prepare_store(store, &msg)?;
    set_working_group(store, &msg_priority);
    insert_msg_in_working_group(store, &msg_byte_size, &msg_id);
    update_working_group(store);
    Ok(())
}

// Store Actions Finish


#[cfg(test)]
mod tests {

    use crate::store::{GroupDefaults, ID, Msg, delete, generate_store, get_next, insert};

    #[test]
    fn should_insert_two_msgs_and_return_the_oldest_highest_msg() {
        let mut store = generate_store();
        let mut msg = Msg {
            priority: 1,
            byte_size: "0123456789".len() as u64
        };
        // insert one document
        let msg_1 = insert(&mut store, &msg).unwrap();
        assert_eq!(store.byte_size, 10);
        assert_eq!(store.groups_map.len(), 1);
        let group = store.groups_map.get(&msg.priority).expect("Collection not found");
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
            byte_size: 100
        };
        assert_eq!(true, insert(&mut store, &msg).is_err());

        store.group_defaults.insert(1, GroupDefaults { max_byte_size: Some(99) });

        assert_eq!(true, insert(&mut store, &msg).is_err());

        msg.byte_size = 99;

        assert_eq!(true, insert(&mut store, &msg).is_ok());

        msg.priority = 0;

        assert_eq!(true, insert(&mut store, &msg).is_err());

    }

    #[test]
    fn should_delete_lower_priority_msgs() {
        let mut store = generate_store();
        store.max_byte_size = Some(10);
        let mut msg = Msg {
            priority: 0,
            byte_size: 10
        };
        let msg_id = insert(&mut store, &msg).unwrap().id;
        let removed_id = insert(&mut store, &msg).unwrap().ids_removed[0];
        let group = store.groups_map.get(&0).unwrap();
        assert_eq!(msg_id, removed_id);
        assert_eq!(10, store.byte_size);
        assert_eq!(10, group.byte_size);
        assert_eq!(1, group.msgs_map.len());

    }

}

