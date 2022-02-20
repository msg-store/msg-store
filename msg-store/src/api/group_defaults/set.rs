use crate::api::config::{update_config, GroupConfig, StoreConfig, ConfigError};
use crate::core::store::{GroupDefaults, Store, StoreError};
use crate::database::DatabaseError;
use crate::api::Database;
use crate::api::file_storage::{rm_from_file_storage, FileStorage, FileStorageError};
use crate::api::stats::Stats;
use std::borrow::BorrowMut;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::Mutex;

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
            write!(f, "SET_GROUP_DEFAULTS_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "SET_GROUP_DEFAULTS_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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
    store_configuration_mutex: &Mutex<StoreConfig>,
    store_configuration_path_option: &Option<PathBuf>,
    priority: u32,
    max_byte_size_option: Option<u64>
) -> Result<(), ApiError> {
    let defaults = GroupDefaults {
        max_byte_size: max_byte_size_option,
    };
    let (pruned_count, msgs_removed) = {
        let mut store = match store_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockingError, err))
        }?;
        match store.update_group_defaults(priority, &defaults) {
            Ok((_bytes_removed, msgs_removed)) => Ok((msgs_removed.len() as u64, msgs_removed)),
            Err(err) => Err(api_error!(ErrTy::StoreError(err)))
        }
    }?;
    for uuid in msgs_removed.into_iter() {
        {
            let mut db = match database_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(api_error!(ErrTy::LockingError, err))
            }?;
            if let Err(err) = db.del(uuid.clone()) {
                return Err(api_error!(ErrTy::DatabaseError(err)))
            }
        }
        if let Some(file_storage_mutex) = file_storage_option {
            let mut file_storage = match file_storage_mutex.lock() {
                Ok(gaurd) => Ok(gaurd),
                Err(err) => Err(api_error!(ErrTy::LockingError, err))
            }?;
            if let Err(err) = rm_from_file_storage(&mut file_storage, &uuid) {
                return Err(api_error!(ErrTy::FileStorageError(err)))
            }
        }
    }
    {
        let mut stats = match stats_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockingError, err))
        }?;
        stats.pruned += pruned_count;
    }
    let mk_group_config = || -> GroupConfig {
        GroupConfig {
            priority,
            max_byte_size: max_byte_size_option,
        }
    };
    {
        let mut config = match store_configuration_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockingError, err))
        }?;
        if let Some(groups) = config.groups.borrow_mut() {
            let mut group_index: Option<usize> = None;
            for i in 0..groups.len() {
                let group = match groups.get(i) {
                    Some(group) => group,
                    None => {
                        continue;
                    }
                };
                if priority == group.priority {
                    group_index = Some(i);
                    break;
                }
            }
            if let Some(index) = group_index {
                if let Some(group) = groups.get_mut(index) {
                    group.max_byte_size = max_byte_size_option;
                } else {
                    groups.push(mk_group_config());
                }
            } else {
                groups.push(mk_group_config());
            }
            groups.sort();
        } else {
            config.groups = Some(vec![mk_group_config()]);
        }
        if let Err(err) = update_config(&config, store_configuration_path_option) {
            return Err(api_error!(ErrTy::ConfigError(err)))
        }
    }    
    Ok(())
}
