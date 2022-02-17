use crate::core::store::{Store, StoreDefaults};
use std::path::PathBuf;
use std::sync::Mutex;
use crate::api::lock;
use crate::api::config::{StoreConfig, update_config};
use crate::api::error_codes::{self, log_err};
use crate::api::file_storage::{FileStorage, rm_from_file_storage};
use crate::api::stats::Stats;

pub fn handle(
    store_mutex: &Mutex<Store>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    store_config_mutex: &Mutex<StoreConfig>,
    store_config_path_option: &Option<PathBuf>,
    max_byte_size: Option<u64>
) -> Result<(), &'static str> {    
    let (prune_count, pruned_uuids) = {
        let mut store = lock(store_mutex)?;        
        store.max_byte_size = max_byte_size;
        let defaults = StoreDefaults {
            max_byte_size,
        };
        match store.update_store_defaults(&defaults) {
            Ok((_bytes_removed, _groups_removed, msgs_removed)) => (msgs_removed.len() as u64, msgs_removed),
            Err(error) => {
                log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                return Err(error_codes::STORE_ERROR)
            }
        }
    };
    if let Some(file_storage_mutex) = file_storage_option {
        let mut file_storage = lock(&file_storage_mutex)?;
        for uuid in pruned_uuids {
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                // log_err(&error.to_string(), file!(), line!(), "");
                return Err("FS ERROR")
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
            log_err(error_codes::COULD_NOT_UPDATE_CONFIGURATION, file!(), line!(), error.to_string());
            return Err(error_codes::COULD_NOT_UPDATE_CONFIGURATION)
        }
    }
    Ok(())
}
