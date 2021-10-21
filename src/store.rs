use std::collections::BTreeMap;

pub type MsgId = i32;
pub type GroupId = i32;
pub type MsgByteSize = i32;
type IdToGroup = BTreeMap<MsgId, GroupId>;

pub struct ImportData {
    pub id: Option<MsgId>,
    pub priority: Option<GroupId>,
    pub byte_size: Option<MsgByteSize>,
    pub msg: Option<Vec<u8>>,
}
pub struct Msg {
    pub priority: GroupId,
    pub byte_size: MsgByteSize,
}
pub struct InsertResult {
    pub id: MsgId,
    pub ids_removed: Vec<MsgId>,
}
pub struct ReinsertResult {
    pub ids_removed: Vec<MsgId>,
}
pub enum MoveResult {
    Ok { insert_result: ReinsertResult, msg_data: Msg },
    Err { error: String },
    InsertionError { insert_result: ReinsertResult, error: String },
    ReinsertionError { insert_error: String, reinsert_error: String }
}
pub struct PruningInfo {
    pub store_difference: Option<MsgByteSize>,
    pub group_difference: Option<MsgByteSize>,
}
pub struct GroupDefaults {
    pub max_byte_size: Option<MsgByteSize>,
}
pub struct WorkingGroup {
    pub priority: GroupId,
    pub group: Group,
}
pub struct Group {
    pub max_byte_size: Option<MsgByteSize>,
    pub byte_size: MsgByteSize,
    pub msgs_map: BTreeMap<MsgId, MsgByteSize>,
}
pub struct Store {
    pub max_byte_size: Option<MsgByteSize>,
    pub byte_size: MsgByteSize,
    pub group_defaults: BTreeMap<GroupId, GroupDefaults>,
    pub next_id: MsgId,
    pub id_to_group_map: IdToGroup,
    pub groups_map: BTreeMap<GroupId, Group>,
    pub working_group: BTreeMap<u8, WorkingGroup>,
    pub msgs_removed: BTreeMap<u8, Vec<MsgId>>
}

pub fn generate_store() -> Store {
    Store {
        max_byte_size: None,
        byte_size: 0,
        group_defaults: BTreeMap::new(),
        next_id: 0,
        id_to_group_map: BTreeMap::new(),
        groups_map: BTreeMap::new(),
        working_group: BTreeMap::new(),
        msgs_removed: BTreeMap::new(),
    }
}

fn set_working_group(store: &mut Store, priority: &GroupId) {
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

fn reject_if_msg_is_too_large_for_group(max_byte_size: &MsgByteSize, msg: &Msg) -> Result<(), String> {
    if &msg.byte_size > max_byte_size {
        return Err("Message is too large for priority group.".to_string());
    }
    Ok(())
}

fn get_bytes_to_remove(max_byte_size: &MsgByteSize, byte_size: &MsgByteSize, msg_byte_size: &MsgByteSize) -> MsgByteSize {
    if byte_size <= msg_byte_size {
        *msg_byte_size
    } else {
        (byte_size + msg_byte_size) - max_byte_size
    }
}

fn should_prune(max_byte_size: &MsgByteSize, new_byte_size: &MsgByteSize) -> bool {
    max_byte_size < new_byte_size
}

fn indentify_msgs_to_remove_from_group(
    msgs_map: &BTreeMap<MsgId, MsgByteSize>,
    bytes_to_remove_from_group: &MsgByteSize,
    bytes_removed_from_group: &mut MsgByteSize,
    msgs_removed: &mut Vec<MsgId>,
) {
    for (id, msg_byte_size) in msgs_map.iter() {
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
    bytes_removed_from_group: &MsgByteSize,
    msgs_removed: &Vec<MsgId>,
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
    max_byte_size: &MsgByteSize,
    msgs_removed: &mut Vec<MsgId>,
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
    msgs_removed: &mut Vec<MsgId>,
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
    bytes_to_remove_from_store: &MsgByteSize,
    msg: &Msg,
) -> MsgByteSize {
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
    bytes_to_remove_from_store: &MsgByteSize,
    bytes_removed_from_store: &mut MsgByteSize,
    msgs_removed: &mut Vec<MsgId>,
) {
    let mut groups_removed: Vec<GroupId> = vec![];
    for (priority, group) in store.groups_map.iter_mut() {
        let mut ids_removed_from_group: Vec<MsgId> = vec![];
        let mut bytes_removed_from_group: MsgByteSize = 0;
        let mut remove_group = true;
        for (id, msg_byte_size) in group.msgs_map.iter() {
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
    bytes_from_lower_priority_groups: &MsgByteSize,
    group_byte_size: &MsgByteSize,
    bytes_to_remove_from_store: &MsgByteSize,
) -> bool {
    &(bytes_from_lower_priority_groups + group_byte_size) < bytes_to_remove_from_store
}

fn reject_if_lacking_priority(
    working_group: &WorkingGroup,
    bytes_from_lower_priority_groups: &MsgByteSize,
    bytes_to_remove_from_store: &MsgByteSize,
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
    msgs_removed: &mut Vec<MsgId>,
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
    let mut msgs_removed: Vec<MsgId> = vec![];
    prune_working_group(store, &mut working_group, msg, &mut msgs_removed)?;
    prune_groups(store, msg, &mut working_group, &mut msgs_removed)?;
    for id in msgs_removed.iter() {
        store.id_to_group_map.remove(id);
    }
    store.msgs_removed.insert(0, msgs_removed);
    store.working_group.insert(0, working_group);
    Ok(())
}

fn insert_msg_in_working_group(store: &mut Store, msg_byte_size: &MsgByteSize, id: &MsgId) {
    let working_group = store.working_group.get_mut(&0).unwrap();
    store.byte_size += msg_byte_size;
    working_group.group.byte_size += msg_byte_size;
    working_group.group.msgs_map.insert(*id, *msg_byte_size);
}

fn generate_insert_result(store: &mut Store, id: MsgId) -> InsertResult {
    InsertResult {
        id,
        ids_removed: store.msgs_removed.remove(&0).unwrap(),
    }
}

pub fn get_next_id(store: &mut Store) -> MsgId {
    store.next_id += 1;
    if store.next_id == MsgId::MAX {
        store.next_id = 0;
    }
    store.next_id
}

fn insert_into_id_to_group_map(store: &mut Store, id: &MsgId, priority: &GroupId) {
    store.id_to_group_map.insert(*id, *priority);
}

fn reinsert(store: &mut Store, msg: &Msg, id: &MsgId) -> Result<ReinsertResult, String> {
    set_working_group(store, &msg.priority);
    prepare_store(store, msg)?;
    insert_msg_in_working_group(store, &msg.byte_size, &id);
    update_working_group(store);
    insert_into_id_to_group_map(store, &id, &msg.priority);
    let result = generate_insert_result(store, *id);
    Ok(ReinsertResult { ids_removed: result.ids_removed })
}

pub fn insert(store: &mut Store, msg: &Msg) -> Result<InsertResult, String> {
    set_working_group(store, &msg.priority);
    prepare_store(store, msg)?;
    let id = get_next_id(store);
    insert_msg_in_working_group(store, &msg.byte_size, &id);
    update_working_group(store);
    insert_into_id_to_group_map(store, &id, &msg.priority);
    let result = generate_insert_result(store, id);
    Ok(result)
}

pub fn delete(store: &mut Store, id: &MsgId) -> Result<(), String> {
    let mut remove_group = false;
    let priority = match store.id_to_group_map.get(&id) {
        Some(priority) => priority,
        None => {
            return Ok(());
        }
    };
    let mut group = match store.groups_map.get_mut(&priority) {
        Some(group) => group,
        None => {
            return Ok(());
        }
    };
    let bytes_removed = match group.msgs_map.remove(&id) {
        Some(bytes_removed) => bytes_removed,
        None => {
            return Ok(());
        }
    };
    group.byte_size -= bytes_removed;
    store.byte_size -= bytes_removed;
    if group.msgs_map.is_empty() {
        remove_group = true;
    }
    if remove_group {
        store.groups_map.remove(&priority);
    }
    store.id_to_group_map.remove(&id);
    Ok(())
}

pub fn get_next(store: &mut Store) -> Result<Option<MsgId>, String> {
    let group = match store.groups_map.iter().rev().next() {
        Some(group) => group,
        None => {
            return Ok(None);
        }
    };
    let id = match group.1.msgs_map.keys().next() {
        Some(id) => *id,
        None => {
            return Ok(None);
        }
    };
    return Ok(Some(id))
}

pub fn get_next_from_group(store: &mut Store, priority: &GroupId) -> Result<Option<MsgId>, String> {
    let group = match store.groups_map.get(&priority) {
        Some(group) => group,
        None => {
            return Ok(None);
        }
    };
    let id = match group.msgs_map.keys().next() {
        Some(id) => *id,
        None => {
            return Ok(None);
        }
    };
    return Ok(Some(id))
}

pub fn msg_exists(store: &mut Store, id: &MsgId) -> Result<bool, String> {
    let priority = match store.id_to_group_map.get(&id) {
        Some(priority) => priority,
        None => {
            return Ok(false);
        }
    };
    let group = match store.groups_map.get(&priority) {
        Some(group) => group,
        None => {
            return Ok(false);
        }
    };
    match group.msgs_map.keys().next() {
        Some(_id) => Ok(true),
        None => Ok(false)
    }
}

pub fn mv(store: &mut Store, id: &MsgId, new_priority: &GroupId) -> MoveResult {
    let priority = match &store.id_to_group_map.get(&id) {
        Some(priority) => **priority,
        None => {
            return MoveResult::Err{error: format!("Id not found.")}
        }
    };
    let group = match store.groups_map.get(&priority) {
        Some(group) => group,
        None => {
            return MoveResult::Err{
                error: format!("Sync Error: id found in id_to_group_map, but the group was found.")
            }
        }
    };
    let byte_size = match group.msgs_map.get(id) {
        Some(byte_size) => *byte_size,
        None => {
            return MoveResult::Err{
                error: format!("Sync Error: id found in id_to_group_map but not in the group.")
            }
        }
    };
    if let Err(error) = delete(store, id) {
        return MoveResult::Err{ error };
    }
    let msg = Msg {
        priority: *new_priority,
        byte_size
    };
    match reinsert(store, &msg, &id) {
        Ok(insert_result) => MoveResult::Ok{ insert_result, msg_data: msg },
        Err(insert_error) => {
            let old_msg = Msg {
                priority,
                byte_size
            };
            match reinsert(store, &old_msg, &id) {
                Ok(insert_result) => {
                    return MoveResult::InsertionError{ insert_result, error: insert_error };
                },
                Err(reinsert_error) => {
                    return MoveResult::ReinsertionError{ insert_error, reinsert_error };
                }
            }
        }
    }
}

// Store Actions Finish

#[cfg(test)]
mod tests {

    use crate::store::{
        delete, generate_store, get_next,
        insert, GroupDefaults, Msg, MsgId, MsgByteSize
    };

    #[test]
    fn should_insert_two_msgs_and_return_the_oldest_highest_msg() {
        let mut store = generate_store();
        let mut msg = Msg {
            priority: 1,
            byte_size: "0123456789".len() as MsgByteSize,
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
        assert_eq!(true, delete(&mut store, &insert_1.id).is_ok());
        assert_eq!(true, delete(&mut store, &insert_2.id).is_ok());
        assert_eq!(0, store.byte_size);
        assert_eq!(0, store.groups_map.len());
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
        let mut ids_inserted: Vec<MsgId> = vec![];
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

}
