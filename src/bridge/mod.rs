use crate::error::ErrorCode;
use crate::msg_data::MsgData;
use crate::uuid::Uuid;
use std::rc::Rc;

pub mod leveldb;
//pub mod sqlite;

pub trait Bridge {
    fn post(&mut self, uuid: Rc<Uuid>, msg: &Vec<u8>) -> Result<(), ErrorCode>;
    fn put(&mut self, uuid: &Uuid, uuid: &Uuid) -> Result<(), ErrorCode>;
    fn get(&self, uuid: Rc<Uuid>) -> Result<Option<Vec<u8>>, ErrorCode>;
    fn del(&mut self, uuid: Rc<Uuid>) -> Result<(), ErrorCode>;
    fn read(&self) -> Result<Vec<MsgData>, ErrorCode>;
}

pub enum BridgeError {
    Open(String),
    Post(String),
    Put(String),
    Get(String),
    Del(String),
}
impl BridgeError {
    pub fn to_error(&self) -> ErrorCode {
        match &self {
            BridgeError::Open(error) => ErrorCode::new(1, "/open", error),
            BridgeError::Post(error) => ErrorCode::new(2, "/post", error),
            BridgeError::Put(error) => ErrorCode::new(3, "/put", error),
            BridgeError::Get(error) => ErrorCode::new(4, "/get", error),
            BridgeError::Del(error) => ErrorCode::new(5, "/delete", error),
        }
    }

    pub fn from_error(error_code: ErrorCode) -> Result<BridgeError, ErrorCode> {
        match error_code.code {
            1 => Ok(BridgeError::Open(error_code.message)),
            2 => Ok(BridgeError::Post(error_code.message)),
            3 => Ok(BridgeError::Put(error_code.message)),
            4 => Ok(BridgeError::Get(error_code.message)),
            5 => Ok(BridgeError::Del(error_code.message)),
            6 => Err(error_code),
            _ => {
                let path = "/parse_error/error_code/code";
                let msg = format!("Cannot format code {}", error_code.code);
                Err(ErrorCode::new(6, path, &msg))
            }
        }
    }
}
