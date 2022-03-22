use crate::config::{StoreConfig, GroupConfig, update_config};
use msg_store::Store;
use log::error;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;
use std::process::exit;
use std::sync::Mutex;

#[derive(Debug)]
pub enum ErrTy {
    // DatabaseError(DatabaseError),
    // FileStorageError(FileStorageError),
    // StoreError(StoreError),
    LockingError
}
impl Display for ErrTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Self::DatabaseError(err) => write!(f, "({})", err),
            // Self::FileStorageError(err) => write!(f, "({})", err),
            // Self::StoreError(err) => write!(f, "({})", err),
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    priority: u16,
}

pub async fn handle(
    store_mutex: &Mutex<Store>,
    configuration_mutex: &Mutex<StoreConfig>,
    configuration_path_option: &Option<PathBuf>, 
    priority: u16
) -> Result<(), ApiError> {
    let mut store = match store_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let mut config = match configuration_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    {
        store.delete_group_defaults(priority);
    }
    {
        let groups = match &config.groups {
            Some(groups) => groups,
            None => {
                return Ok(());
            }
        };
        let new_groups: Vec<GroupConfig> = groups
            .iter()
            .filter(|group| {
                if group.priority != priority {
                    true
                } else {
                    false
                }
            })
            .map(|group| group.clone())
            .collect();
        config.groups = Some(new_groups);
        if let Err(error) = update_config(&config, configuration_path_option) {
            error!("ERROR_CODE: 11d80bf2-5b87-436f-9c26-914ca2718347. Could not update config file: {}", error);
            exit(1);
        }
    }
    Ok(())
}
