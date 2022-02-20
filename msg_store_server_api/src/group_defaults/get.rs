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
            write!(f, "GET_GROUP_DEFAULTS_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "GET_GROUP_DEFAULTS_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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


#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct GroupDefaults {
    pub priority: u32,
    pub max_byte_size: Option<u64>,
}

pub async fn handle(
    store_mutex: &Mutex<Store>,
    priority_option: Option<u32>
) -> Result<Vec<GroupDefaults>, ApiError> {
    let store = match store_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    if let Some(priority) = priority_option {
        if let Some(defaults) = store.group_defaults.get(&priority) {
            let group_defaults = GroupDefaults {
                priority: priority.clone(),
                max_byte_size: defaults.max_byte_size,
            };
            Ok(vec![group_defaults])
        } else {
            Ok(vec![])
        }
    } else {
        let data = store
            .group_defaults
            .iter()
            .map(|(priority, defaults)| GroupDefaults {
                priority: priority.clone(),
                max_byte_size: defaults.max_byte_size,
            })
            .collect::<Vec<GroupDefaults>>();
        Ok(data)
    }
}
