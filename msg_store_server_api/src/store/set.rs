use msg_store::{Store, StoreDefaults, StoreError};
use msg_store_database_plugin::DatabaseError;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Mutex;
use crate::Database;
use crate::config::{StoreConfig, update_config, ConfigError};
use crate::file_storage::{FileStorage, rm_from_file_storage, FileStorageError};
use crate::stats::Stats;

#[derive(Debug)]
pub enum ErrTy {
    ConfigError(ConfigError),
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    StoreError(StoreError),
    LockingError
}
impl Display for ErrTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigError(err) => write!(f, "({})", err),
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
            write!(f, "SET_STORE_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "SET_STORE_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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

pub async fn handle(
    store_mutex: &Mutex<Store>,
    database_mx: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    store_config_mutex: &Mutex<StoreConfig>,
    store_config_path_option: &Option<PathBuf>,
    max_byte_size: Option<u64>
) -> Result<(), ApiError> {
    let mut store = match store_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let mut database = match database_mx.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let mut stats = match stats_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let mut config = match store_config_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let (prune_count, pruned_uuids) = {
        store.max_byte_size = max_byte_size;
        let defaults = StoreDefaults {
            max_byte_size,
        };
        match store.update_store_defaults(&defaults) {
            Ok((_bytes_removed, _groups_removed, msgs_removed)) => Ok((msgs_removed.len() as u64, msgs_removed)),
            Err(err) => Err(api_error!(ErrTy::StoreError(err)))
        }
    }?;
    {
        for uuid in &pruned_uuids {
            if let Err(err) = database.del(uuid.clone()) {
                return Err(api_error!(ErrTy::DatabaseError(err)))
            }
        }
    }
    if let Some(file_storage_mutex) = file_storage_option {
        let mut file_storage = match file_storage_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockingError, err))
        }?;
        for uuid in pruned_uuids {
            if let Err(err) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(api_error!(ErrTy::FileStorageError(err)))
            }
        }
    }
    {
        stats.pruned += prune_count;
    }
    {
        config.max_byte_size = max_byte_size;
        if let Err(err) = update_config(&mut config, store_config_path_option) {
            return Err(api_error!(ErrTy::ConfigError(err)))
        }
    }
    Ok(())
}
