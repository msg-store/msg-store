
use crate::{
    api_err,
    api::{
        ApiError,
        ApiErrorTy,
        NoErr,
        lock,
        Database,
        file_storage::{
            rm_from_file_storage,
            FileStorage
        },
        stats::Stats
    },
    core::store::Store,
    core::uuid::Uuid
};
use std::error::Error;
use std::sync::{Arc, Mutex};

pub fn try_rm<E: Error>(
    store_mutex: &Mutex<Store>, 
    database_mutex: &Mutex<Database<E>>, 
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    priority: u32) -> Result<(), ApiError<E, NoErr>> {
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
                return Err(api_err!(ApiErrorTy::StoreError(error)));
            }
        }
        {
            let mut db = lock(&database_mutex)?;
            if let Err(error) = db.del(uuid.clone()) {
                return Err(api_err!(ApiErrorTy::DbError(error)));
            }
        }
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error) = rm_from_file_storage(&mut file_storage, uuid) {
                return Err(api_err!(ApiErrorTy::FileStorageError(error)))
            }
        }
        deleted_count += 1;
    }
    let mut stats = lock(&stats_mutex)?;
    stats.deleted += deleted_count;
    Ok(())
}
