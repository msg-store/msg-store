use crate::database::Db;
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

pub mod config {
    use log::error;
    use serde::{Deserialize, Serialize};
    use serde_json::{from_str as from_json_str, to_string_pretty as to_json_string};
    use std::{
        fs::{self, read_to_string},
        path::{Path, PathBuf},
        process::exit
    };

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct GroupConfig {
        pub priority: u32,
        pub max_byte_size: Option<u32>,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct StoreConfig {
        pub host: Option<String>,
        pub port: Option<u32>,
        pub node_id: Option<u16>,
        pub database: Option<String>,
        pub leveldb_path: Option<PathBuf>,
        pub file_storage: Option<bool>,
        pub file_storage_path: Option<PathBuf>,
        pub max_byte_size: Option<u32>,
        pub groups: Option<Vec<GroupConfig>>,
        pub no_update: Option<bool>,
        pub update: Option<bool>
    }

    impl StoreConfig {
        pub fn new() -> StoreConfig {
            StoreConfig {
                host: Some("127.0.0.1".to_string()),
                port: Some(8080),
                node_id: None,
                database: Some("mem".to_string()),
                leveldb_path: None,
                file_storage: Some(false),
                file_storage_path: None,
                max_byte_size: None,
                groups: None,
                no_update: None,
                update: Some(true)
            }
        }
        pub fn open(config_path: &Path) -> StoreConfig {
            let contents: String = match read_to_string(config_path) {
                Ok(content) => content,
                Err(error) => {
                    error!("ERROR_CODE: 9cf571a9-fe29-4737-a977-d5ad580e1b28. Could not read configuration: {}", error.to_string());
                    exit(1);
                }
            };
            match from_json_str(&contents) {
                Ok(config) => config,
                Err(error) => {
                    error!("ERROR_CODE: 95d9ece7-39bb-41fb-93a7-a530786673d3. Could not parse configuration: {}", error.to_string());
                    exit(1);
                }
            }
        }
        pub fn update_config_file(&self, config_path: &Path) -> Result<(), String> {
            let contents = match to_json_string(&self) {
                Ok(contents) => Ok(contents),
                Err(error) => Err(error.to_string()),
            }?;
            if let Err(error) = fs::write(config_path, contents) {
                return Err(error.to_string());
            };
            Ok(())
        }
        pub fn to_json(&self) -> Result<String, String> {
            match to_json_string(&self) {
                Ok(json) => Ok(json),
                Err(error) => Err(error.to_string())
            }
        }
        pub fn inherit(&mut self, configuration: Self) {
            self.host = configuration.host;
            self.port = configuration.port;
            self.node_id = configuration.node_id;
            self.database = configuration.database;
            self.leveldb_path = configuration.leveldb_path;
            self.file_storage = configuration.file_storage;
            self.file_storage_path = configuration.file_storage_path;
            self.max_byte_size = configuration.max_byte_size;
            self.groups = configuration.groups;
            self.no_update = configuration.no_update;
        }
    }

    pub fn update_config(config: &StoreConfig, config_path: &Option<PathBuf>) -> Result<(), String> {
        let should_update = {
            let mut should_update = true;
            if let Some(no_update) = config.no_update {
                if no_update {
                    should_update = false;
                }
            }
            should_update
        };
        if should_update {
            if let Some(config_path) = config_path {
                config.update_config_file(&config_path)?;
            }
        }
        Ok(())
    }

}
