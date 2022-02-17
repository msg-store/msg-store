use crate::api::configuration::{update_config, GroupConfig, StoreConfig};
use crate::core::store::{GroupDefaults, Store};
use crate::api::{ApiErrorTy, ApiError, NoErr, lock, Database};
use crate::api::file_storage::{rm_from_file_storage, FileStorage};
use crate::api::stats::Stats;
use crate::api_err;
use std::borrow::BorrowMut;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Mutex;

pub fn try_set<E: Error>(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database<E>>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    store_configuration_mutex: &Mutex<StoreConfig>,
    store_configuration_path_option: &Option<PathBuf>,
    priority: u32,
    max_byte_size_option: Option<u32>
) -> Result<(), ApiError<E, NoErr>> {
    let defaults = GroupDefaults {
        max_byte_size: max_byte_size_option,
    };
    let (pruned_count, msgs_removed) = {
        let mut store = lock(store_mutex)?;
        match store.update_group_defaults(priority, &defaults) {
            Ok((_bytes_removed, msgs_removed)) => Ok((msgs_removed.len() as u32, msgs_removed)),
            Err(error) => Err(api_err!(ApiErrorTy::StoreError(error)))
        }
    }?;
    for uuid in msgs_removed.into_iter() {
        {
            let mut db = lock(database_mutex)?;
            if let Err(error) = db.del(uuid.clone()) {
                return Err(api_err!(ApiErrorTy::DbError(error)))
            }
        }
        if let Some(file_storage_mutex) = file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(api_err!(ApiErrorTy::FileStorageError(error)))
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
            return Err(api_err!(ApiErrorTy::ConfigurationError(error)))
        }
    }    
    Ok(())
}
