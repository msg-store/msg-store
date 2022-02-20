use msg_store::Store;
use serde::{Deserialize, Serialize};
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
            write!(f, "GET_GROUP_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "GET_GROUP_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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
#[serde(rename_all = "camelCase")]
pub struct Info {
    priority: Option<u32>,
    include_msg_data: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Msg {
    uuid: String,
    byte_size: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    priority: u32,
    byte_size: u64,
    max_byte_size: Option<u64>,
    msg_count: u64,
    messages: Vec<Msg>,
}

pub fn handle(
    store_mutex: &Mutex<Store>,
    priority_option: Option<u32>,
    include_msg_data: bool
) -> Result<Vec<Group>, ApiError> {
    let store = match store_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    if let Some(priority) = priority_option {
        if let Some(group) = store.groups_map.get(&priority) {
            let group = Group {
                priority: priority.clone(),
                byte_size: group.byte_size,
                max_byte_size: group.max_byte_size,
                msg_count: group.msgs_map.len() as u64,
                messages: match include_msg_data {
                    true => group
                        .msgs_map
                        .iter()
                        .map(|(uuid, byte_size)| Msg {
                            uuid: uuid.to_string(),
                            byte_size: byte_size.clone(),
                        })
                        .collect::<Vec<Msg>>(),
                    false => vec![]
                }
            };
            Ok(vec![group])
        } else {
            Ok(vec![])
        }
    } else {
        let data = store
            .groups_map
            .iter()
            .map(|(priority, group)| Group {
                priority: priority.clone(),
                byte_size: group.byte_size,
                max_byte_size: group.max_byte_size,
                msg_count: group.msgs_map.len() as u64,
                messages: match include_msg_data {
                    true => group
                        .msgs_map
                        .iter()
                        .map(|(uuid, byte_size)| Msg {
                            uuid: uuid.to_string(),
                            byte_size: byte_size.clone(),
                        })
                        .collect::<Vec<Msg>>(),
                    false => vec![]
                },
            })
            .collect::<Vec<Group>>();
        Ok(data)
    }
}
