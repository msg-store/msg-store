use crate::bridge::{Bridge, BridgeError};
use crate::msg_data::MsgData;
use crate::error::ErrorCode;
use std::path::{Path, PathBuf};
use sqlite::Connection;

pub struct SqliteDb {
    connection: Connection
}
impl SqliteDb {
    pub fn new(connection: Connection) -> SqliteDb {
        SqliteDb { connection }
    }
}

impl Bridge for SqliteDb {
    fn post(&mut self, uuid: Rc<Uuid>, msg: Vec<u8>) -> Result<(), BridgeError> {
    }
}
