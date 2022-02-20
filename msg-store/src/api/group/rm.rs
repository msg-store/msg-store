use crate::core::store::{Store, StoreError};
use crate::core::uuid::Uuid;
use crate::database::DatabaseError;
use crate::api::Database;
use crate::api::file_storage::{rm_from_file_storage, FileStorage, FileStorageError};
use crate::api::stats::Stats;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum ErrTy {
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    StoreError(StoreError),
    LockingError
}
impl Display for ErrTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::LockingError => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub err_ty: ErrTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "REMOVE_GROUP_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "REMOVE_GROUP_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! api_error {
    ($err_ty:expr) => {
        ApiError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        ApiError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub fn handle(
    store_mutex: &Mutex<Store>, 
    database_mutex: &Mutex<Database>, 
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    priority: u32) -> Result<(), ApiError> {
    let list = {
        let store = match store_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockingError, err))
        }?;
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
            let mut store = match store_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(api_error!(ErrTy::LockingError, err))
            }?;
            if let Err(err) = store.del(uuid.clone()) {
                return Err(api_error!(ErrTy::StoreError(err)));
            }
        }
        {
            let mut db = match database_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(api_error!(ErrTy::LockingError, err))
            }?;
            if let Err(err) = db.del(uuid.clone()) {
                return Err(api_error!(ErrTy::DatabaseError(err)));
            }
        }
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = match file_storage_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(api_error!(ErrTy::LockingError, err))
            }?;
            if let Err(err) = rm_from_file_storage(&mut file_storage, uuid) {
                return Err(api_error!(ErrTy::FileStorageError(err)))
            }
        }
        deleted_count += 1;
    }
    let mut stats = match stats_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    stats.deleted += deleted_count;
    Ok(())
}
