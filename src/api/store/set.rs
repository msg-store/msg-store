use crate::core::store::{Store, StoreDefaults};
use std::path::PathBuf;
use std::sync::Mutex;
use crate::api::{ApiError, ApiErrorTy, NoErr, lock};
use crate::api::configuration::{StoreConfig, update_config};
use crate::api::file_storage::{FileStorage, rm_from_file_storage};
use crate::api::stats::Stats;
use crate::api_err;

pub fn try_set(
    store_mutex: &Mutex<Store>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    store_config_mutex: &Mutex<StoreConfig>,
    store_config_path_option: &Option<PathBuf>,
    max_byte_size: Option<u32>
) -> Result<(), ApiError<NoErr, NoErr>> {    
    let (prune_count, pruned_uuids) = {
        let mut store = lock(store_mutex)?;        
        store.max_byte_size = max_byte_size;
        let defaults = StoreDefaults {
            max_byte_size,
        };
        match store.update_store_defaults(&defaults) {
            Ok((_bytes_removed, _groups_removed, msgs_removed)) => Ok((msgs_removed.len() as u32, msgs_removed)),
            Err(error) => Err(api_err!(ApiErrorTy::StoreError(error)))
        }
    }?;
    if let Some(file_storage_mutex) = file_storage_option {
        let mut file_storage = lock(&file_storage_mutex)?;
        for uuid in pruned_uuids {
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(api_err!(ApiErrorTy::FileStorageError(error)))
            }
        }
    }
    {
        let mut stats = lock(stats_mutex)?;
        stats.pruned += prune_count;
    }
    {
        let mut config = lock(store_config_mutex)?;
        config.max_byte_size = max_byte_size;
        if let Err(error) = update_config(&mut config, store_config_path_option) {
            return Err(api_err!(ApiErrorTy::ConfigurationError(error)));
        }
    }
    Ok(())
}
