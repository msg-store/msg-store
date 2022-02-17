use crate::api::{
    lock,
    Database,
    stats::Stats
};
use crate::api::error_codes;
use crate::api::file_storage::{rm_from_file_storage, FileStorage};
use crate::core::store::Store;
use crate::core::uuid::Uuid;
use std::sync::{Arc, Mutex};

pub fn handle(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    uuid: Arc<Uuid>) -> Result<(), &'static str> {
    {
        let mut store = lock(&store_mutex)?;
        if let Err(error) = store.del(uuid.clone()) {
            error_codes::log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
            return Err(error_codes::STORE_ERROR)
        }
    }
    {
        let mut db = lock(&database_mutex)?;
        if let Err(error) = db.del(uuid.clone()) {
            error_codes::log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
            return Err(error_codes::DATABASE_ERROR)
        }
    }
    {
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                // error_codes::log_err(&error.to_string(), file!(), line!(), "");
                return Err("FS ERROR")
            }
        }
    }
    {
        let mut stats = lock(&stats_mutex)?;
        stats.deleted += 1;
    }
    Ok(())
}
