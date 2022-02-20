use crate::{
    Database,
    stats::Stats
};
use crate::file_storage::{rm_from_file_storage, FileStorage, FileStorageError};
use msg_store::{Store, StoreError};
use msg_store_uuid::Uuid;
use msg_store_database_plugin::DatabaseError;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum RemoveErrorTy {
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    StoreError(StoreError),
    CouldNotFindFileStorage,
    LockingError,
    CouldNotGetNextChunkFromPayload,
    CouldNotParseChunk
}
impl Display for RemoveErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::CouldNotFindFileStorage |
            Self::LockingError |
            Self::CouldNotGetNextChunkFromPayload |
            Self::CouldNotParseChunk => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct RemoveError {
    pub err_ty: RemoveErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "REMOVE_MSG_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "REMOVE_MSG_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! rm_msg_error {
    ($err_ty:expr) => {
        RemoveError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        RemoveError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub async fn handle(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    uuid: Arc<Uuid>) -> Result<(), RemoveError> {
    {
        let mut store = match store_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(rm_msg_error!(RemoveErrorTy::LockingError, err))
        }?;
        if let Err(error) = store.del(uuid.clone()) {
            return Err(rm_msg_error!(RemoveErrorTy::StoreError(error)))
        }
    }
    {
        let mut db = match database_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(rm_msg_error!(RemoveErrorTy::LockingError, err))
        }?;
        if let Err(error) = db.del(uuid.clone()) {
            return Err(rm_msg_error!(RemoveErrorTy::DatabaseError(error)));
        }
    }
    {
        if let Some(file_storage_mutex) = &file_storage_option {
            let mut file_storage = match file_storage_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(rm_msg_error!(RemoveErrorTy::LockingError, err))
            }?;
            if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(rm_msg_error!(RemoveErrorTy::FileStorageError(error)))
            }
        }
    }
    {
        let mut stats = match stats_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(rm_msg_error!(RemoveErrorTy::LockingError, err))
        }?;
        stats.deleted += 1;
    }
    Ok(())
}
