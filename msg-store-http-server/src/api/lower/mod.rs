pub use log::error;
pub use msg_store::{Store as MsgStore, Uuid};
use msg_store_db_plugin::Db;
use std::sync::{Mutex, MutexGuard};

pub mod export;
pub mod file_storage;
pub mod group;
pub mod group_defaults;
pub mod msg;
pub mod stats;
pub mod store;

pub mod error_codes {

    use log::error;
    use std::fmt::Display;

    pub type Str = &'static str;

    /// Fatal Errors
    pub const COULD_NOT_CREATE_DIRECTORY: Str = "COULD_NOT_CREATE_DIRECTORY";
    pub const DIRECTORY_DOES_NOT_EXIST: Str = "DIRECTORY_DOES_NOT_EXIST";
    pub const PATH_IS_NOT_A_DIRECTORY: Str = "PATH_IS_NOT_A_DIRECTORY";
    pub const COULD_NOT_READ_DIRECTORY: Str = "COULD_NOT_READ_DIRECTORY";
    pub const COULD_NOT_OPEN_FILE: Str = "COULD_NOT_OPEN_FILE";
    pub const COULD_NOT_CREATE_FILE: Str = "COULD_NOT_CREATE_FILE";
    pub const COULD_NOT_WRITE_TO_FILE: Str = "COULD_NOT_WRITE_TO_FILE";
    pub const COULD_NOT_GET_METADATA: Str = "COULD_NOT_GET_METADATA";
    pub const COULD_NOT_REMOVE_FILE: Str = "COULD_NOT_REMOVE_FILE";
    pub const STORE_ERROR: Str = "STORE_ERROR";
    pub const DATABASE_ERROR: Str = "DATABASE_ERROR";
    pub const COULD_NOT_LOCK_ITEM: Str = "COULD_NOT_LOCK_ITEM";
    pub const COULD_NOT_FIND_FILE_STORAGE: Str = "COULD_NOT_FIND_FILE_STORAGE";
    pub const COULD_NOT_UPDATE_CONFIGURATION: Str = "COULD_NOT_UPDATE_CONFIGURATION";
    pub const COULD_NOT_READ_BUFFER: Str = "COULD_NOT_READ_BUFFER";
    pub const INVALID_DATABASE_OPTION: Str = "INVALID_DATABASE_OPTION";
    pub const COULD_NOT_COPY_FILE: Str = "COULD_NOT_COPY_FILE";
    pub const INVAILD_MSG: Str = "INVAILD_MSG";
    pub const INVALID_PATH: Str = "INVALID_PATH";

    /// Non-Fatal Errors
    pub const COULD_NOT_GET_CHUNK_FROM_PAYLOAD: Str = "COULD_NOT_GET_CHUNK_FROM_PAYLOAD";
    pub const COULD_NOT_PARSE_CHUNK: Str = "COULD_NOT_PARSE_CHUNK";
    pub const MISSING_HEADERS: Str = "MISSING_HEADERS";
    pub const MALFORMED_HEADERS: Str = "MALFORMED_HEADERS";
    pub const FILE_STORAGE_NOT_CONFIGURED: Str = "FILE_STORAGE_NOT_CONFIGURED";
    pub const PAYLOAD_ERROR: Str = "PAYLOAD_ERROR";

    /// Messaging Errors
    pub const INVALID_PRIORITY: Str = "INVALID_PRIORITY";
    pub const MISSING_PRIORITY: Str = "MISSING_PRIORITY";
    pub const INVALID_BYTESIZE_OVERRIDE: Str = "INVALID_BYTESIZE_OVERRIDE";
    pub const MISSING_BYTESIZE_OVERRIDE: Str = "MISSING_BYTESIZE_OVERRIDE";
    pub const INVALID_UUID: Str = "INVALID_UUID";

    /// Store Errors
    pub const MSG_EXCEEDES_STORE_MAX: Str = "EXCEEDES_STORE_MAX";
    pub const MSG_EXCEEDES_GROUP_MAX: Str = "EXCEEDES_GROUP_MAX";
    pub const MSG_LACKS_PRIORITY: Str = "MSG_LACKS_PRIORITY";

    /// Store Fatal Errors
    pub const SYNC_ERROR: Str = "SYNC_ERROR";

    pub fn could_not_create_directory<T: Display + Into<String>>(file: Str, line: u32, error: T) -> Str {
        log_err(COULD_NOT_CREATE_DIRECTORY, file, line, error);
        COULD_NOT_CREATE_DIRECTORY
    }

    pub fn could_not_get_chunk_from_payload<T: Display + Into<String>>(file: Str, line: u32, error: T) -> Str {
        log_err(COULD_NOT_GET_CHUNK_FROM_PAYLOAD, file, line, error);
        COULD_NOT_GET_CHUNK_FROM_PAYLOAD
    }

    pub fn could_not_parse_chunk<T: Display + Into<String>>(file: Str, line: u32, error: T) -> Str {
        log_err(COULD_NOT_PARSE_CHUNK, file, line, error);
        COULD_NOT_PARSE_CHUNK
    }

    pub fn store_error<T: Display + Into<String>>(file: Str, line: u32, error: T) -> Str {
        log_err(STORE_ERROR, file, line, error);
        STORE_ERROR
    }

    pub fn database_error<T: Display + Into<String>>(file: Str, line: u32, error: T) -> Str {
        log_err(DATABASE_ERROR, file, line, error);
        DATABASE_ERROR
    }

    pub fn log_err<T: Display + Into<String>>(error_code: &'static str, file: &'static str, line: u32, msg: T) {
        let msg = format!("ERROR_CODE: {}. file: {}. line: {}. {}", error_code, file, line, msg);
        error!("{}", msg.trim());
    }

}

pub type Database = Box<dyn Db>;
pub enum Either<A, B> {
    A(A),
    B(B)
}

pub fn lock<'a, T: Send + Sync>(item: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, &'static str> {
    match item.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(error) => {
            error_codes::log_err(error_codes::COULD_NOT_LOCK_ITEM, file!(), line!(), error.to_string());
            Err(error_codes::COULD_NOT_LOCK_ITEM)
        }
    }
}
