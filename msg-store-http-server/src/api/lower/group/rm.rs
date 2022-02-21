use crate::{
    api::{
        lower::{
            lock,
            Database,
            error_codes,
            file_storage::{
                rm_from_file_storage,
                FileStorage
            },
            stats::Stats
        }
    }
};
use msg_store::{Store, Uuid};
use std::sync::{Arc, Mutex};

pub fn handle(
    store_mutex: &Mutex<Store>, 
    database_mutex: &Mutex<Database>, 
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    priority: u32) -> Result<(), &'static str> {
    let list = {
        let store = lock(&store_mutex)?;
        // get list of messages to remove
        let list = if let Some(group) = store.groups_map.get(&priority) {
            group
                .msgs_map
                .keys()
                .map(|uuid| { uuid.clone() })
                .collect::<Vec<Arc<Uuid>>>()
        } else {
            return Ok(());
        };
        list
    };
    let mut deleted_count = 0;
    for uuid in list.iter() {
        {
            let mut store = lock(&store_mutex)?;
            if let Err(error) = store.del(uuid.clone()) {
                error_codes::log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                return Err(error_codes::STORE_ERROR);
            }
        }
        {
            let mut db = lock(&database_mutex)?;
            if let Err(error) = db.del(uuid.clone()) {
                error_codes::log_err(error_codes::DATABASE_ERROR, file!(), line!(), error);
                return Err(error_codes::DATABASE_ERROR);
            }
        }
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error_code) = rm_from_file_storage(&mut file_storage, uuid) {
                error_codes::log_err(error_code, file!(), line!(), "");
                return Err(error_code)
            }
        }
        deleted_count += 1;
    }
    let mut stats = lock(&stats_mutex)?;
    stats.deleted += deleted_count;
    Ok(())
}
