use msg_store_database_plugin::Db;
pub mod export;
pub mod file_storage;
pub mod group;
pub mod group_defaults;
pub mod msg;
pub mod stats;
pub mod store;

pub type Database = Box<dyn Db>;
pub enum Either<A, B> {
    A(A),
    B(B)
}
impl<A, B> Either<A, B> {
    pub fn a(self) -> A {
        match self {
            Self::A(inner) => inner,
            Self::B(_) => panic!("Item is not A")
        }
    }
    pub fn b(self) -> B {
        match self {
            Self::A(_) => panic!("Item is not B"),
            Self::B(inner) => inner
        }
    }
}

pub mod config {
    use serde::{Deserialize, Serialize};
    use serde_json::{from_str as from_json_str, to_string_pretty as to_json_string};
    use std::fmt::Display;
    use std::fs::{self, read_to_string};
    use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ConfigErrorTy {
    CouldNotConvertToJson,
    CouldNotParseJson,
    CouldNotReadFile,
    CouldNotWriteToFile
}
impl Display for ConfigErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Self::CouldNotCreateDirectory |
            // Self::CouldNotCreateFile |
            // Self::CouldNotGetChunkFromPayload |
            // Self::CouldNotReadDirectory |
            // Self::CouldNotReadMetadata |
            // Self::CouldNotRemoveFile |
            // Self::CouldNotOpenFile |
            Self::CouldNotParseJson |
            Self::CouldNotWriteToFile |
            &Self::CouldNotReadFile |
            // Self::DirectoryDoesNotExist |
            Self::CouldNotConvertToJson => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct ConfigError {
    pub err_ty: ConfigErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CONFIGURATION_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)?;
        if let Some(msg) = &self.msg {
            write!(f, "{}", msg)
        } else {
            Ok(())
        }
    }   
}

macro_rules! config_error {
    ($err_ty:expr) => {
        ConfigError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        ConfigError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}
    

    #[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub struct GroupConfig {
        pub priority: u16,
        pub max_byte_size: Option<u64>,
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
        pub max_byte_size: Option<u64>,
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
        pub fn open(config_path: &Path) -> Result<StoreConfig, ConfigError> {
            let contents: String = match read_to_string(config_path) {
                Ok(content) => Ok(content),
                Err(err) => Err(config_error!(ConfigErrorTy::CouldNotReadFile, err))
            }?;
            match from_json_str(&contents) {
                Ok(config) => Ok(config),
                Err(err) => Err(config_error!(ConfigErrorTy::CouldNotParseJson, err))
            }
        }
        pub fn update_config_file(&self, config_path: &Path) -> Result<(), ConfigError> {
            let contents = match to_json_string(&self) {
                Ok(contents) => Ok(contents),
                Err(err) => Err(config_error!(ConfigErrorTy::CouldNotConvertToJson, err)),
            }?;
            if let Err(err) = fs::write(config_path, contents) {
                return Err(config_error!(ConfigErrorTy::CouldNotWriteToFile, err));
            };
            Ok(())
        }
        pub fn to_json(&self) -> Result<String, ConfigError> {
            match to_json_string(&self) {
                Ok(json) => Ok(json),
                Err(err) => Err(config_error!(ConfigErrorTy::CouldNotConvertToJson, err))
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

}
