use crate::{
    api::{
        lower::{
            lock
        }
    }
};
use msg_store::Store;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Info {
    priority: Option<u32>,
    include_msg_data: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Msg {
    uuid: String,
    byte_size: u32,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    priority: u32,
    byte_size: u32,
    max_byte_size: Option<u32>,
    msg_count: u32,
    messages: Vec<Msg>,
}

pub fn handle(
    store_mutex: &Mutex<Store>,
    priority_option: Option<u32>,
    include_msg_data: bool
) -> Result<Vec<Group>, &'static str> {
    let store = lock(&store_mutex)?;
    if let Some(priority) = priority_option {
        if let Some(group) = store.groups_map.get(&priority) {
            let group = Group {
                priority: priority.clone(),
                byte_size: group.byte_size,
                max_byte_size: group.max_byte_size,
                msg_count: group.msgs_map.len() as u32,
                messages: match include_msg_data {
                    true => group
                        .msgs_map
                        .iter()
                        .map(|(uuid, byte_size)| Msg {
                            uuid: uuid.to_string(),
                            byte_size: byte_size.clone(),
                        })
                        .collect::<Vec<Msg>>(),
                    false => vec![]
                }
            };
            Ok(vec![group])
        } else {
            Ok(vec![])
        }
    } else {
        let data = store
            .groups_map
            .iter()
            .map(|(priority, group)| Group {
                priority: priority.clone(),
                byte_size: group.byte_size,
                max_byte_size: group.max_byte_size,
                msg_count: group.msgs_map.len() as u32,
                messages: match include_msg_data {
                    true => group
                        .msgs_map
                        .iter()
                        .map(|(uuid, byte_size)| Msg {
                            uuid: uuid.to_string(),
                            byte_size: byte_size.clone(),
                        })
                        .collect::<Vec<Msg>>(),
                    false => vec![]
                },
            })
            .collect::<Vec<Group>>();
        Ok(data)
    }
}
