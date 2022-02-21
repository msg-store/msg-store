use msg_store::Store;
use serde::Serialize;
use std::sync::Mutex;
use super::super::lock;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefaults {
    priority: u32,
    max_byte_size: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupData {
    priority: u32,
    byte_size: u32,
    max_byte_size: Option<u32>,
    msg_count: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreData {
    byte_size: u32,
    max_byte_size: Option<u32>,
    msg_count: usize,
    group_count: usize,
    groups: Vec<GroupData>,
    group_defaults: Vec<GroupDefaults>,
}

pub fn handle(store_mutex: &Mutex<Store>) -> Result<StoreData, &'static str> {
    let store = lock(store_mutex)?;
    let groups = store
        .groups_map
        .iter()
        .map(|(priority, group)| GroupData {
            priority: *priority,
            byte_size: group.byte_size,
            max_byte_size: group.max_byte_size,
            msg_count: group.msgs_map.len(),
        })
        .collect::<Vec<GroupData>>();
    let group_defaults = store
        .group_defaults
        .iter()
        .map(|(priority, details)| GroupDefaults {
            priority: *priority,
            max_byte_size: details.max_byte_size,
        })
        .collect::<Vec<GroupDefaults>>();
    let data = StoreData {
        byte_size: store.byte_size,
        max_byte_size: store.max_byte_size,
        msg_count: store.id_to_group_map.len(),
        group_count: store.groups_map.len(),
        groups,
        group_defaults,
    };
    Ok(data)
}
