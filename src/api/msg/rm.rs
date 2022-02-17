use crate::api::{
    lock,
    Database,
    stats::Stats
};
use crate::api::{ApiError, ApiErrorTy, NoErr};
use crate::api::file_storage::{rm_from_file_storage, FileStorage};
use crate::core::store::Store;
use crate::core::uuid::Uuid;
use crate::api_err;
use std::error::Error;
use std::sync::{Arc, Mutex};

pub fn try_rm<E: Error>(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database<E>>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    uuid: Arc<Uuid>) -> Result<(), ApiError<E, NoErr>> {
    {
        let mut store = lock(&store_mutex)?;
        if let Err(error) = store.del(uuid.clone()) {
            return Err(api_err!(ApiErrorTy::StoreError(error)) )
        }
    }
    {
        let mut db = lock(&database_mutex)?;
        if let Err(error) = db.del(uuid.clone()) {
            return Err(api_err!(ApiErrorTy::DbError(error)))
        }
    }
    {
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = lock(file_storage_mutex)?;
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(api_err!(ApiErrorTy::FileStorageError(error)))
            }
        }
    }
    {
        let mut stats = lock(&stats_mutex)?;
        stats.deleted += 1;
    }
    Ok(())
}
