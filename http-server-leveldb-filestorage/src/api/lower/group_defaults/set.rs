use crate::api::update_config;
use crate::api::lower::{lock, Database};
use crate::api::lower::error_codes;
use crate::api::lower::file_storage::{rm_from_file_storage, FileStorage};
use crate::api::lower::stats::Stats;
use crate::config::GroupConfig;
use crate::StoreConfig;
use msg_store::Store;
use msg_store::store::GroupDefaults;
use std::borrow::BorrowMut;
use std::path::PathBuf;
use std::sync::Mutex;

pub fn handle(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    store_configuration_mutex: &Mutex<StoreConfig>,
    store_configuration_path_option: &Option<PathBuf>,
    priority: u32,
    max_byte_size_option: Option<u32>
) -> Result<(), &'static str> {
    let defaults = GroupDefaults {
        max_byte_size: max_byte_size_option,
    };
    let (pruned_count, msgs_removed) = {
        let mut store = lock(store_mutex)?;
        match store.update_group_defaults(priority, &defaults) {
            Ok((_bytes_removed, msgs_removed)) => (msgs_removed.len() as u32, msgs_removed),
            Err(error) => {
                error_codes::log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                return Err(error_codes::STORE_ERROR)
            }
        }
    };
    for uuid in msgs_removed.into_iter() {
        {
            let mut db = lock(database_mutex)?;
            if let Err(error) = db.del(uuid.clone()) {
                error_codes::log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                return Err(error_codes::DATABASE_ERROR)
            }
        }
        if let Some(file_storage_mutex) = file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error_code) = rm_from_file_storage(&mut file_storage, &uuid) {
                error_codes::log_err(error_code, file!(), line!(), "");
                return Err(error_code)
            }
        }
    }
    {
        let mut stats = lock(stats_mutex)?;
        stats.pruned += pruned_count;
    }
    let mk_group_config = || -> GroupConfig {
        GroupConfig {
            priority,
            max_byte_size: max_byte_size_option,
        }
    };
    {
        let mut config = lock(store_configuration_mutex)?;
        if let Some(groups) = config.groups.borrow_mut() {
            let mut group_index: Option<usize> = None;
            for i in 0..groups.len() {
                let group = match groups.get(i) {
                    Some(group) => group,
                    None => {
                        continue;
                    }
                };
                if priority == group.priority {
                    group_index = Some(i);
                    break;
                }
            }
            if let Some(index) = group_index {
                if let Some(group) = groups.get_mut(index) {
                    group.max_byte_size = max_byte_size_option;
                } else {
                    groups.push(mk_group_config());
                }
            } else {
                groups.push(mk_group_config());
            }
            groups.sort();
        } else {
            config.groups = Some(vec![mk_group_config()]);
        }
        if let Err(error) = update_config(&config, store_configuration_path_option) {
            error_codes::log_err(error_codes::COULD_NOT_UPDATE_CONFIGURATION, file!(), line!(), error);
            return Err(error_codes::COULD_NOT_UPDATE_CONFIGURATION)
        }
    }    
    Ok(())
}
