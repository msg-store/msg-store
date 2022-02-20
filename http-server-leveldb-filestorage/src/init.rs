use clap::{App, Arg};
use dirs::home_dir;
use msg_store::{Store, StoreDefaults, GroupDefaults, StoreError};
use msg_store_database_plugin::{Db, DatabaseError};
use msg_store_database_in_memory_plugin::MemDb;
use msg_store_database_leveldb_plugin::Leveldb;
use server_api::file_storage::{
    FileStorage,
    FileStorageError,
    read_file_storage_direcotory,
    discover_files,
    rm_from_file_storage
};
use server_api::stats::Stats;
use server_api::config::{StoreConfig, ConfigError};
use std::fmt::Display;
use std::fs::create_dir_all;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct InitResult {
    pub host: String,
    pub store: Mutex<Store>,
    pub configuration: Mutex<StoreConfig>,
    pub db: Mutex<Box<dyn Db>>,
    pub configuration_path: Option<PathBuf>,
    pub file_storage: Option<Mutex<FileStorage>>,
    pub stats: Mutex<Stats>
}

const HOST: &'static str = "host";
const PORT: &'static str = "port";
const CONFIG: &'static str = "config";
const CONFIG_PATH: &'static str = "config-path";
const UPDATE: &'static str = "update";
const NO_UPDATE: &'static str = "no-update";
const DATABASE: &'static str = "database";
const LEVELDB_PATH: &'static str = "leveldb-path";
const FILE_STORAGE: &'static str = "file-storage";
const FILE_STORAGE_PATH: &'static str = "file-storage-path";
const NODE_ID: &'static str = "node-id";

#[derive(Debug)]
pub enum InitErrorTy {
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    ConfigError(ConfigError),
    StoreError(StoreError),
    CouldNotCreateDatabaseDirectory,
    CouldNotCreateDatabasePath,
    CouldNotCreateFileStoragePath,
    CouldNotCreateMsgStoreDirectory,
    CouldNotWriteToConfigurationFile,
    InvalidDatabaseOption,
    InvalidNodeId,
    InvalidPortOption,
    MissingLeveldbPath,
    UpdateOptionConflict
}
impl Display for InitErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::ConfigError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::CouldNotCreateDatabaseDirectory |
            Self::CouldNotCreateDatabasePath |
            Self::CouldNotCreateFileStoragePath |
            Self::CouldNotCreateMsgStoreDirectory |
            Self::CouldNotWriteToConfigurationFile |
            Self::InvalidDatabaseOption |
            Self::InvalidNodeId |
            Self::InvalidPortOption |
            Self::MissingLeveldbPath |
            Self::UpdateOptionConflict => write!(f, "({:#?})", self)
        }
    }
}

#[derive(Debug)]
pub struct InitError {
    pub err_ty: InitErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "SERVER_INIT_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "SERVER_INIT_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! init_error {
    ($err_ty:expr) => {
        InitError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        InitError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

fn get_app<'a>() -> App<'a, 'a> {
    App::new("msg-store-server")
        .version("0.1.0")
        .author("Joshua Enokson <kilograhm@pm.me>")
        .about("A priority message store")
        .arg(
            Arg::with_name(HOST)
                .short("h")
                .long(HOST)
                .help("Sets the host address"),
        )
        .arg(
            Arg::with_name(PORT)
                .short("p")
                .long(PORT)
                .help("Sets the port number"),
        )
        .arg(
            Arg::with_name(CONFIG)
                .short("c")
                .long(CONFIG)
                .help("Use a configuration a file. Will create one at $HOME/.msg-store/.config.json if one does not exist."),
        )
        .arg(
            Arg::with_name(CONFIG_PATH)
                // .short("c")
                .long(CONFIG_PATH)
                .takes_value(true)
                .help("Sets a path for the config file")
        )
        .arg(
            Arg::with_name(NO_UPDATE)
                .long(NO_UPDATE)
                .conflicts_with(UPDATE)
                .help("Will not write updated config to disk"),
        )
        .arg(
            Arg::with_name(UPDATE)
                .long(UPDATE)
                .conflicts_with(NO_UPDATE)
                .help("Will write updated config to disk"),
        )
        .arg(
            Arg::with_name(DATABASE)
                .short("d")
                .long(DATABASE)
                .takes_value(true)
                .help("Determines the database to use. (memory or leveldb)")
        )
        .arg(
            Arg::with_name(LEVELDB_PATH)
                .long(LEVELDB_PATH)
                .help("Sets the leveldb database path")
                .takes_value(true)
        )
        .arg(
            Arg::with_name(FILE_STORAGE)
                .short("f")
                .long(FILE_STORAGE)
                .help("Sets the location of the messages held in files"),
        )
        .arg(
            Arg::with_name(FILE_STORAGE_PATH)
                // .short("f")
                .long(FILE_STORAGE_PATH)
                .takes_value(true)
                .help("Sets the location of the messages held in files"),
        )
        .arg(
            Arg::with_name(NODE_ID)
                // .short("f")
                .long(NODE_ID)
                .takes_value(true)
                .help("Sets the node id of the msg-store"),
        )
}

pub fn init() -> Result<InitResult, InitError> {

    let matches = get_app().get_matches();

    // configuration
    // get configuration path
    let mut configuration = StoreConfig::new();
    let configuration_path = {
        // get saved configuration path
        let configuration_path = {
            if let Some(configuration_path) = matches.value_of(CONFIG_PATH) {
                Some(PathBuf::from(configuration_path))
            } else {
                if matches.is_present(CONFIG) {
                    if let Some(home_dir) = home_dir() {
                        let mut configuration_path = home_dir;
                        configuration_path.push(".msg-store");
                        if let Err(error) = create_dir_all(&configuration_path) {
                            return Err(init_error!(InitErrorTy::CouldNotCreateMsgStoreDirectory, error))
                        }
                        configuration_path.push("config.json");                        
                        // create default configuration file
                        let defualt_configuration = StoreConfig::new();
                        if let Err(error) = defualt_configuration.update_config_file(&configuration_path) {
                            return Err(init_error!(InitErrorTy::CouldNotWriteToConfigurationFile, error));
                        };
                        Some(configuration_path)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
        };
        configuration_path
    };

    // get update config from saved config
    if let Some(configuration_path) = configuration_path.as_ref() {
        let saved_configuration = match StoreConfig::open(configuration_path) {
            Ok(config) => Ok(config),
            Err(err) => Err(init_error!(InitErrorTy::ConfigError(err)))
        }?;
        // validate values from configuration file
        if let Some(database) = &saved_configuration.database {
            let database = database.to_ascii_lowercase();
            if database != "mem" && database != "memory" && database != "leveldb" {
                return Err(init_error!(InitErrorTy::InvalidDatabaseOption, "Expected mem, memory or leveldb"));
            }
        }
        if let Some(no_update) = configuration.no_update {
            if let Some(update) = configuration.update {
                if update == no_update {
                    return Err(init_error!(InitErrorTy::UpdateOptionConflict, "The option '--no-update' cannot be used with '--update'"));
                }
            }
        }
        configuration.inherit(saved_configuration);
    }

    // update config from cli options
    // get host, port, no-update, database, leveldb-path, file-storage, file-storage-path
    // update host from cli
    let host = {
        if let Some(host) = matches.value_of(HOST) {
            let host = host.to_string();
            configuration.host = Some(host.to_string());
            host
        } else {
            if let Some(host) = &configuration.host {
                host.to_string()
            } else {
                "127.0.0.1".to_string()
            }
        }
    };    
    // update port from cli
    let port = {
        if let Some(port) = matches.value_of(PORT) {
            let port = match port.parse::<u32>() {
                Ok(port) => Ok(port),
                Err(_error) => Err(init_error!(InitErrorTy::InvalidPortOption, "Expected u32"))
            }?;
            configuration.port = Some(port);
            port
        } else {
            if let Some(port) = configuration.port {
                port
            } else {
                8080
            }
        }
    };
    // update no-update from cli
    if matches.is_present(NO_UPDATE) {
        configuration.no_update = Some(true);
    }
    // update update from cli
    if matches.is_present(UPDATE) {
        configuration.no_update = Some(true);
    }
    // validate update/no-update options
    if let Some(no_update) = configuration.no_update {
        if let Some(update) = configuration.update {
            if update == no_update {
                return Err(init_error!(InitErrorTy::UpdateOptionConflict, "The option '--no-update' cannot be used with '--update'"));
            }
        }
    }
    // update database, leveldb-path from cli
    if let Some(database) = matches.value_of(DATABASE) {
        // validate database option
        let database_lower = database.to_ascii_lowercase();
        if database_lower == "leveldb" {
            // validate leveldb path option
            let leveldb_path = {
                if let Some(database_path) = matches.value_of(LEVELDB_PATH) {
                    PathBuf::from(database_path)
                } else {
                    if let Some(level_db_path) = configuration.leveldb_path {
                        PathBuf::from(level_db_path)
                    } else {
                        if let Some(home_dir) = home_dir() {
                            let mut level_db_path = home_dir;
                            level_db_path.push(".msg-store/leveldb");
                            level_db_path
                        } else {
                            return Err(init_error!(InitErrorTy::CouldNotCreateDatabaseDirectory))
                        }
                    }
                }
            };
            if let Err(error) = create_dir_all(&leveldb_path) {
                return Err(init_error!(InitErrorTy::CouldNotCreateDatabasePath, error));
            }
            configuration.leveldb_path = Some(leveldb_path);
            configuration.database = Some("leveldb".to_string());

        } else if database_lower == "mem" || database_lower == "memory" {
            configuration.database = Some("memory".to_string());
        } else {
            return Err(init_error!(InitErrorTy::InvalidDatabaseOption, "Expected mem, memory or leveldb"));
        }
    }
    // update file-storage, file-storage-path from cli
    if let Some(file_storage_path) = matches.value_of(FILE_STORAGE_PATH) {
        configuration.file_storage_path = Some(PathBuf::from(file_storage_path));
    } else {
        if matches.is_present(FILE_STORAGE) {
            if let Some(home_dir) = home_dir() {
                let mut file_storage_path = home_dir;
                file_storage_path.push(".msg-store/file-storage");
                if let Err(error) = create_dir_all(&file_storage_path) {
                    return Err(init_error!(InitErrorTy::CouldNotCreateFileStoragePath, error));
                }
                configuration.file_storage_path = Some(file_storage_path);
            } else {
                return Err(init_error!(InitErrorTy::CouldNotCreateFileStoragePath, "Home directory does not exist"));
            }
        }
    }
    // get database
    let mut database: Box<dyn Db> = {
        if let Some(database_type) = &configuration.database {
            let database_type = database_type.to_ascii_lowercase();
            if database_type == "mem" || database_type == "memory" {
                Box::new(MemDb::new())
            } else if database_type == "leveldb" {
                let level_db_path = match &configuration.leveldb_path {
                    Some(level_db_path) => Ok(level_db_path),
                    None => Err(init_error!(InitErrorTy::MissingLeveldbPath))
                }?;
                let leveldb = match Leveldb::new(level_db_path) {
                    Ok(leveldb) => Ok(leveldb),
                    Err(error) => Err(init_error!(InitErrorTy::DatabaseError(error)))
                }?;
                Box::new(leveldb)
            } else {
                return Err(init_error!(InitErrorTy::InvalidDatabaseOption));
            }
        } else {
            return Err(init_error!(InitErrorTy::InvalidDatabaseOption));
        }
    };
    // get the stored messages from the database
    let msgs = match database.fetch() {
        Ok(msgs) => Ok(msgs),
        Err(error) => Err(init_error!(InitErrorTy::DatabaseError(error)))
    }?;
    // get file list of all files stored on disk
    let mut file_storage: Option<FileStorage> = {
        if let Some(file_storage_path) = &configuration.file_storage_path {
            if let Err(error) = create_dir_all(file_storage_path) {
                return Err(init_error!(InitErrorTy::CouldNotCreateFileStoragePath, error));
            }
            let mut file_storage = match FileStorage::new(file_storage_path) {
                Ok(file_storage) => Ok(file_storage),
                Err(err) => Err(init_error!(InitErrorTy::FileStorageError(err)))
            }?;
            let uuids = match read_file_storage_direcotory(file_storage_path) {
                Ok(index) => index,
                Err(error) => {
                    return Err(init_error!(InitErrorTy::FileStorageError(error)));
                }
            };
            discover_files(&mut file_storage, uuids);
            Some(file_storage)
        } else {
            None
        }
    };
    // get node_id
    // update configuration only if match is found
    if let Some(node_id_str) = matches.value_of(NODE_ID) {
        let node_id = match node_id_str.parse::<u16>() {
            Ok(node_id) => Ok(node_id),
            Err(error) => Err(init_error!(InitErrorTy::InvalidNodeId, error))
        }?;
        configuration.node_id = Some(node_id);
    }

    let mut store = match Store::new(configuration.node_id) {
        Ok(store) => Ok(store),
        Err(error) => Err(init_error!(InitErrorTy::StoreError(error)))
    }?;

    if let Some(max_byte_size) = configuration.max_byte_size {
        if let Err(error) = store.update_store_defaults(&StoreDefaults { max_byte_size: Some(max_byte_size) }) {
            return Err(init_error!(InitErrorTy::StoreError(error)));
        }
    }

    if let Some(groups) = &configuration.groups {
        for group in groups.iter() {
            if let Err(error) = store.update_group_defaults(group.priority, &GroupDefaults { max_byte_size: group.max_byte_size }) {
                return Err(init_error!(InitErrorTy::StoreError(error)));
            };
        }
    }

    // add messages to store, prune excess
    let (removed_uuids, pruned_count) = {
        let mut removed_uuids = vec![];
        let mut pruned_count = 0;
        for (uuid, msg_byte_size) in &msgs {
            let mut add_result = match store.add_with_uuid(uuid.clone(), *msg_byte_size) {
                Ok(add_result) => add_result,
                Err(error) => {
                    // TODO: add error handling options to the configuration
                    return Err(init_error!(InitErrorTy::StoreError(error)));
                }
            };
            for uuid_removed in &add_result.msgs_removed {
                if let Err(error) = database.del(uuid_removed.clone()) {
                    return Err(init_error!(InitErrorTy::DatabaseError(error)));
                }
            }
            removed_uuids.append(&mut add_result.msgs_removed);
            pruned_count += add_result.msgs_removed.len();
        }
        (removed_uuids, pruned_count as u64)
    };
    // removed pruned files if any
    if let Some(file_storage) = file_storage.as_mut() {
        for uuid in removed_uuids {
            if let Err(error) = rm_from_file_storage(file_storage, &uuid) {
                return Err(init_error!(InitErrorTy::FileStorageError(error)));
            }
        }
    }
    let stats = Stats { inserted: 0, deleted: 0, pruned: pruned_count };
    Ok(InitResult {
        host: format!("{}:{}", host, port),
        store: Mutex::new(store),
        db: Mutex::new(database),
        file_storage: match file_storage {
            Some(file_storage) => Some(Mutex::new(file_storage)),
            None => None
        },
        configuration: Mutex::new(configuration),
        configuration_path,
        stats: Mutex::new(stats)
    })

}
