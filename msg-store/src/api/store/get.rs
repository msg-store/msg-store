use crate::core::store::Store;
use serde::Serialize;
use std::fmt::Display;
use std::sync::Mutex;

#[derive(Debug)]
pub enum ErrTy {
    LockingError
}
impl Display for ErrTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
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
            write!(f, "GET_STORE_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "GET_STORE_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefaults {
    priority: u32,
    max_byte_size: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GroupData {
    priority: u32,
    byte_size: u64,
    max_byte_size: Option<u64>,
    msg_count: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StoreData {
    byte_size: u64,
    max_byte_size: Option<u64>,
    msg_count: usize,
    group_count: usize,
    groups: Vec<GroupData>,
    group_defaults: Vec<GroupDefaults>,
}

pub fn handle(store_mutex: &Mutex<Store>) -> Result<StoreData, ApiError> {
    let store = match store_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let groups = store
        .groups_map
        .iter()
        .map(|(priority, group)| GroupData {
            priority: *priority,
            byte_size: group.byte_size,
            max_byte_size: group.max_byte_size,
            msg_count: group.msgs_map.len(),
        })
        .collect::<Vec<GroupData>>();
    let group_defaults = store
        .group_defaults
        .iter()
        .map(|(priority, details)| GroupDefaults {
            priority: *priority,
            max_byte_size: details.max_byte_size,
        })
        .collect::<Vec<GroupDefaults>>();
    let data = StoreData {
        byte_size: store.byte_size,
        max_byte_size: store.max_byte_size,
        msg_count: store.id_to_group_map.len(),
        group_count: store.groups_map.len(),
        groups,
        group_defaults,
    };
    Ok(data)
}
