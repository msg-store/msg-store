use crate::api::update_config;
use crate::api::lower::lock;
use crate::config::GroupConfig;
use crate::StoreConfig;
use log::error;
use msg_store::Store;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::exit;
use std::sync::Mutex;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    priority: u32,
}

pub fn handle(
    store_mutex: &Mutex<Store>,
    configuration_mutex: &Mutex<StoreConfig>,
    configuration_path_option: &Option<PathBuf>, 
    priority: u32) -> Result<(), &'static str> {
    {
        let mut store = lock(&store_mutex)?;
        store.delete_group_defaults(priority);
    }
    {
        let mut config = lock(&configuration_mutex)?;
        let groups = match &config.groups {
            Some(groups) => groups,
            None => {
                return Ok(());
            }
        };
        let new_groups: Vec<GroupConfig> = groups
            .iter()
            .filter(|group| {
                if group.priority != priority {
                    true
                } else {
                    false
                }
            })
            .map(|group| group.clone())
            .collect();
        config.groups = Some(new_groups);
        if let Err(error) = update_config(&config, configuration_path_option) {
            error!("ERROR_CODE: 11d80bf2-5b87-436f-9c26-914ca2718347. Could not update config file: {}", error);
            exit(1);
        }
    }
    Ok(())
}
