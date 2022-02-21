use crate::StoreConfig;
use crate::api::lower::{lock, Database};
use crate::api::lower::error_codes::{self,log_err};
use crate::api::lower::file_storage::{
    FileStorage,
    create_directory,
    get_file_path_from_id,
    rm_from_file_storage
};
use crate::api::lower::stats::Stats;
use msg_store::Store;
use msg_store_db_plugin::Db;
use msg_store_plugin_leveldb::Leveldb;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::fs::{OpenOptions, copy, remove_file, create_dir_all};
use std::io::Write;
use std::sync::Mutex;

/// Creates a export directory, appending an integer to create a unique directory if needed
pub fn get_export_destination_directory(destination_directory: &Path) -> PathBuf {
    let mut finalized_path = destination_directory.to_path_buf();
    if destination_directory.exists() {
        // if it exists, then append a number to the path and check if it too exits.
        // repeat until a non-existing path is found        
        let mut count = 1;
        loop {
            finalized_path.push(format!("msg-store-backup-{}", count));
            if !finalized_path.exists() {
                break;
            }
            finalized_path.pop();
            count += 1;
        }
    }
    finalized_path
}

pub fn create_export_directory(export_directory: &Path) -> Result<bool, &'static str> {
    if export_directory.exists() {
        if let Err(error) = create_dir_all(export_directory) {
            log_err(error_codes::COULD_NOT_CREATE_DIRECTORY, file!(), line!(), error.to_string());
            return Err(error_codes::COULD_NOT_CREATE_DIRECTORY)
        }
        return Ok(true)
    }
    Ok(false)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StoredPacket {
    pub uuid: String,
    pub msg: String,
}

pub fn handle(
    store_mutex: &Mutex<Store>,
    database_mutex: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    stats_mutex: &Mutex<Stats>,
    configuration_mutex: &Mutex<StoreConfig>,
    export_directory: &Path
) -> Result<(), &'static str> {

    let max_count = {
        let store = lock(store_mutex)?;
        store.id_to_group_map.len()
    };

    let database = {
        let configuration = lock(configuration_mutex)?;
        if let Some(database) = &configuration.database {
            let lower_case = database.to_ascii_lowercase();
            if lower_case == "mem" || lower_case == "memory" {
                "mem"
            } else if lower_case == "leveldb" {
                "leveldb"
            } else {
                log_err(error_codes::INVALID_DATABASE_OPTION, file!(), line!(), "");
                return Err(error_codes::INVALID_DATABASE_OPTION);
            }
        } else {
            "mem"
        }
    };
    let deleted_count = {
        let mut deleted_count = 0;
        if database == "mem" {
            if let Err(error_code) = create_export_directory(&export_directory) {
                log_err(error_code, file!(), line!(), "");
                return Err(error_code);
            }
            let mut file_path = export_directory.to_path_buf();
            // get the default file name
            file_path.push("msg-store-backup.txt");
            // open the file
            let mut file = match OpenOptions::new()
                .append(true)
                .create_new(true)
                .open(file_path)
            {
                Ok(file) => file,
                Err(error) => {
                    log_err(error_codes::COULD_NOT_OPEN_FILE, file!(), line!(), error.to_string());
                    return Err(error_codes::COULD_NOT_OPEN_FILE);
                }
            };
            // get the number of messages to export
            for _i in 0..max_count {
                // get a uuid
                let uuid = {
                    let store = lock(store_mutex)?;
                    match store.get(None, None, false) {
                        Ok(uuid) => uuid,
                        Err(error) => {
                            log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::STORE_ERROR);
                        }
                    }            
                };
                // if no uuid is found then break the loop
                let uuid = if let Some(uuid) = uuid {
                    uuid
                } else {
                    break;
                };
                // get the message from the database    
                let msg = {
                    let mut db = lock(database_mutex)?;
                    match db.get(uuid.clone()) {
                        Ok(msg) => msg,
                        Err(error) => {
                            log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::DATABASE_ERROR);
                        }
                    }
                };
                // convert the message to string (for some reason? This may not be needed.)
                let msg = match String::from_utf8(msg.to_vec()) {
                    Ok(msg) => msg,
                    Err(error) => {
                        log_err(error_codes::INVAILD_MSG, file!(), line!(), error.to_string());
                        return Err(error_codes::INVAILD_MSG);
                    }
                };
                // package up the data
                let transformed_stored_packet = StoredPacket {
                    uuid: uuid.to_string(),
                    msg,
                };
                let mut packet_string = json!(transformed_stored_packet).to_string();
                packet_string.push_str("\n");
                if let Err(error) = file.write_all(packet_string.as_bytes()) {
                    log_err(error_codes::COULD_NOT_WRITE_TO_FILE, file!(), line!(), error.to_string());
                    return Err(error_codes::COULD_NOT_WRITE_TO_FILE);
                };
                // remove message from store
                {
                    let mut store = lock(store_mutex)?;
                    if let Err(error) = store.del(uuid.clone()) {
                        log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                        return Err(error_codes::STORE_ERROR);
                    }
                }
                // remove message from database
                {
                    let mut db = lock(database_mutex)?;
                    if let Err(error) = db.del(uuid) {
                        log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                        return Err(error_codes::DATABASE_ERROR);
                    }
                }
                deleted_count += 1;
            }
        } else if database == "leveldb" {
            // convert the string into a pathbuf
            let export_dir_path = get_export_destination_directory(&export_directory);

            // get the leveldb path
            let mut leveldb_path = export_dir_path.to_path_buf();
            leveldb_path.push("leveldb");
            // open the leveldb instance
            let mut leveldb_backup = match Leveldb::new(&leveldb_path) {
                Ok(leveldb) => leveldb,
                Err(error) => {
                    log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                    return Err(error_codes::DATABASE_ERROR);
                }
            };

            if let Some(file_storage_mutex) = file_storage_option {

                // create file storage directory
                if let Err(error_code) = create_directory(&export_dir_path) {
                    log_err(error_code, file!(), line!(), "");
                    return Err(error_code);
                }
                let file_storage_export_directory = match create_directory(&export_dir_path) {
                    Ok(directory) => directory,
                    Err(error_code) => {
                        log_err(error_code, file!(), line!(), "");
                        return Err(error_code);
                    }
                };

                for _ in 0..max_count {
                    let store = lock(store_mutex)?;
                    let mut leveldb = lock(database_mutex)?;
                    let mut file_storage = lock(&file_storage_mutex)?;
                    let uuid = match store.get(None, None, false) {
                        Ok(uuid) => uuid,
                        Err(error) => {
                            log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::STORE_ERROR);
                        }
                    };
                    let uuid = match uuid {
                        Some(uuid) => uuid,
                        None => { break }
                    };
                    let msg = match leveldb.get(uuid.clone()) {
                        Ok(msg) => msg,
                        Err(error) => {
                            log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::DATABASE_ERROR);
                        }
                    };                
                    let msg_byte_size = msg.len() as u32;
    
                    let src_file_path = get_file_path_from_id(&file_storage.path, &uuid);
                    let dest_file_path = get_file_path_from_id(&file_storage_export_directory, &uuid);
                    if let Err(error) = copy(&src_file_path, &dest_file_path) {
                        log_err(error_codes::COULD_NOT_COPY_FILE, file!(), line!(), error.to_string());
                        return Err(error_codes::COULD_NOT_COPY_FILE);
                    };
                    // remove the file from the index
                    if let Err(error_code) = rm_from_file_storage(&mut file_storage, &uuid) {
                        log_err(error_code, file!(), line!(), "");
                        return Err(error_code);
                    }

                    // add the data to the leveldb backup
                    // if it errors then copy the destination file back to the source
                    // dont exit until on error handling has finished
                    if let Err(error) = leveldb_backup.add(uuid, msg, msg_byte_size) {
                        log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());                        
                        if let Err(error) = copy(&dest_file_path, &src_file_path) {
                            log_err(error_codes::COULD_NOT_COPY_FILE, file!(), line!(), error.to_string());
                        };
                        if let Err(error) = remove_file(dest_file_path) {
                            log_err(error_codes::COULD_NOT_REMOVE_FILE, file!(), line!(), error.to_string());
                        }
                        return Err(error_codes::DATABASE_ERROR);
                    }
                    // update deleted count
                    deleted_count += 1;    
                }
            } else {
                for _ in 0..max_count {
                    let store = lock(store_mutex)?;
                    let mut leveldb = lock(database_mutex)?;
                    let uuid = match store.get(None, None, false) {
                        Ok(uuid) => uuid,
                        Err(error) => {
                            log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::STORE_ERROR);
                        }
                    };
                    let uuid = match uuid {
                        Some(uuid) => uuid,
                        None => { break }
                    };
                    let msg = match leveldb.get(uuid.clone()) {
                        Ok(msg) => msg,
                        Err(error) => {
                            log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                            return Err(error_codes::DATABASE_ERROR);
                        }
                    };                
                    let msg_byte_size = msg.len() as u32;

                    // add the data to the leveldb backup
                    // if it errors then copy the destination file back to the source
                    // dont exit until on error handling has finished
                    if let Err(error) = leveldb_backup.add(uuid, msg, msg_byte_size) {
                        log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                        return Err(error_codes::DATABASE_ERROR);
                    }
                    // update deleted count
                    deleted_count += 1;    
                }
            }
        } else {
            log_err(error_codes::INVALID_DATABASE_OPTION, file!(), line!(), "");
            return Err(error_codes::INVALID_DATABASE_OPTION);
        }
        deleted_count
    };
    // update stats
    {
        let mut stats = lock(stats_mutex)?;
        stats.deleted += deleted_count;
    }    
    Ok(())
}
