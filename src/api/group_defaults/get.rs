use crate::api::{lock, ApiError, NoErr};
use crate::core::store::Store;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefaults {
    priority: u32,
    max_byte_size: Option<u32>,
}

pub fn try_get(
    store_mutex: &Mutex<Store>,
    priority_option: Option<u32>
) -> Result<Vec<GroupDefaults>, ApiError<NoErr, NoErr>> {
    let store = lock(store_mutex)?;
    if let Some(priority) = priority_option {
        if let Some(defaults) = store.group_defaults.get(&priority) {
            let group_defaults = GroupDefaults {
                priority: priority.clone(),
                max_byte_size: defaults.max_byte_size,
            };
            Ok(vec![group_defaults])
        } else {
            Ok(vec![])
        }
    } else {
        let data = store
            .group_defaults
            .iter()
            .map(|(priority, defaults)| GroupDefaults {
                priority: priority.clone(),
                max_byte_size: defaults.max_byte_size,
            })
            .collect::<Vec<GroupDefaults>>();
        Ok(data)
    }
}
