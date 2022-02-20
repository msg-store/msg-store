use crate::{
    api::{
        lower::{
            lock,
            Database,
            stats::Stats,
            error_codes,
            file_storage::{rm_from_file_storage, FileStorage}
        }
    }
};
use msg_store::{Store, Uuid};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex}
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    uuid: String,
}

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
            if let Err(error_code) = rm_from_file_storage(&mut file_storage, &uuid) {
                error_codes::log_err(error_code, file!(), line!(), "");
                return Err(error_code)
            }
        }
    }
    {
        let mut stats = lock(&stats_mutex)?;
        stats.deleted += 1;
    }
    Ok(())
}