use serde::{Deserialize, Serialize};
use serde_json::{from_str as from_json_str, to_string_pretty as to_json_string};
use std::{
    fs::{self, read_to_string},
    path::{Path, PathBuf}
};
use std::error::Error;
use std::fmt::Display;

#[derive(Debug, PartialEq, Eq)]
pub enum ConfigErrorTy {
    CouldNotReadFile,
    CouldNotParse,
    CouldNotConvertToJson,
    CouldNotWriteToFile
}
impl Display for ConfigErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigError {
    pub error_ty: ConfigErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}
impl ConfigError {
    pub fn new(error_ty: ConfigErrorTy, file: &'static str, line: u32, msg: Option<String>) -> ConfigError {
        ConfigError {
            error_ty,
            file,
            line,
            msg
        }
    }
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self)
    }
}

impl Error for ConfigError {

}

macro_rules! config_err {
    ($err_ty:expr) => {
        ConfigError::new($err_ty, file!(), line!(), None)
    };
    ($err_ty:expr, $msg:expr) => {
        ConfigError::new($err_ty, file!(), line!(), Some($msg.to_string()))
    };
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupConfig {
    pub priority: u32,
    pub max_byte_size: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoreConfig {
    pub host: Option<String>,
    pub port: Option<u32>,
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
    pub fn open(config_path: &Path) -> Result<StoreConfig, ConfigError> {
        let contents: String = match read_to_string(config_path) {
            Ok(content) => Ok(content),
            Err(error) => Err(config_err!(ConfigErrorTy::CouldNotReadFile, error))
        }?;
        match from_json_str(&contents) {
            Ok(config) => Ok(config),
            Err(error) => Err(config_err!(ConfigErrorTy::CouldNotParse, error))
        }
    }
    pub fn update_config_file(&self, config_path: &Path) -> Result<(), ConfigError> {
        let contents = match to_json_string(&self) {
            Ok(contents) => Ok(contents),
            Err(error) => Err(config_err!(ConfigErrorTy::CouldNotConvertToJson, error)),
        }?;
        if let Err(error) = fs::write(config_path, contents) {
            return Err(config_err!(ConfigErrorTy::CouldNotWriteToFile, error));
        };
        Ok(())
    }
    pub fn to_json(&self) -> Result<String, ConfigError> {
        match to_json_string(&self) {
            Ok(json) => Ok(json),
            Err(error) => Err(config_err!(ConfigErrorTy::CouldNotConvertToJson, error))
        }
    }
    pub fn inherit(&mut self, configuration: Self) {
        self.host = configuration.host;
        self.port = configuration.port;
        self.database = configuration.database;
        self.leveldb_path = configuration.leveldb_path;
        self.file_storage = configuration.file_storage;
        self.file_storage_path = configuration.file_storage_path;
        self.max_byte_size = configuration.max_byte_size;
        self.groups = configuration.groups;
        self.no_update = configuration.no_update;
    }
}

pub fn update_config(config: &StoreConfig, config_path: &Option<PathBuf>) -> Result<(), ConfigError> {
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