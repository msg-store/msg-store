use crate::api::{lock, Database};
use crate::api::file_storage::{
    FileStorage,
    FileStorageError,
    create_directory,
    get_file_path_from_id,
    rm_from_file_storage
};
use crate::api::stats::Stats;
use crate::core::store::{Store, StoreError};
use crate::database::leveldb::{Db, Leveldb, DatabaseError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fmt::Display;
use std::fs::{copy, remove_file, create_dir_all};
use std::sync::Mutex;

#[derive(Debug)]
pub enum ExportErrorTy {
    CouldNotAddFileToBackup(DatabaseError),
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    StoreError(StoreError),
    CouldNotCopyFile,
    CouldNotCreateDirectory,
    CouldNotReinsertFileAfterError,
    CouldNotRemoveFileAfterError,
    LockError
}
impl Display for ExportErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::CouldNotAddFileToBackup(err) => write!(f, "({})", err),
            Self::CouldNotCopyFile |
            Self::CouldNotCreateDirectory |
            Self::CouldNotReinsertFileAfterError |
            Self::CouldNotRemoveFileAfterError |
            Self::LockError => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct ExportError {
    pub err_ty: ExportErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "EXPORT_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "EXPORT_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! export_error {
    ($err_ty:expr) => {
        ExportError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        ExportError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

/// Creates a export directory, appending an integer to create a unique directory if needed
fn get_export_destination_directory(destination_directory: &Path) -> PathBuf {
    let mut finalized_path = destination_directory.to_path_buf();
    if destination_directory.exists() {
        // if it exists, then append a number to the path and check if it too exits.
        // repeat until a non-existing path is found        
        let mut count = 1;
        loop {
            finalized_path = PathBuf::from(format!("{}-{}", finalized_path.to_str().unwrap(), count));
            // finalized_path = PathBuf::new(format!("{}-{}", finalized_path.to_str().unwrap(), count));
            if !finalized_path.exists() {
                break;
            }
            finalized_path.pop();
            count += 1;
        }
    }
    finalized_path
}

fn create_export_directory(export_directory: &Path) -> Result<bool, ExportError> {
    if export_directory.exists() {
        if let Err(error) = create_dir_all(export_directory) {
            return Err(export_error!(ExportErrorTy::CouldNotCreateDirectory, error))
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
    export_directory: &Path
) -> Result<(), ExportError> {

    let max_count = {
        let store = match lock(store_mutex) {
            Ok(gaurd) => Ok(gaurd),
            Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
        }?;
        store.id_to_group_map.len()
    };

    let deleted_count = {
        let mut deleted_count = 0;
        // convert the string into a pathbuf
        let export_dir_path = get_export_destination_directory(&export_directory);

        create_export_directory(&export_dir_path)?;

        // get the leveldb path
        let mut leveldb_path = export_dir_path.to_path_buf();
        leveldb_path.push("leveldb");

        if let Err(error) = create_dir_all(&leveldb_path) {
            return Err(export_error!(ExportErrorTy::CouldNotCreateDirectory, error))
        }

        // open the leveldb instance
        let mut leveldb_backup = match Leveldb::new(&leveldb_path) {
            Ok(leveldb) => Ok(leveldb),
            Err(error) => Err(export_error!(ExportErrorTy::DatabaseError(error)))
        }?;

        if let Some(file_storage_mutex) = file_storage_option {

            // create file storage directory
            if let Err(error) = create_directory(&export_dir_path) {
                return Err(export_error!(ExportErrorTy::FileStorageError(error)));
            }
            let file_storage_export_directory = match create_directory(&export_dir_path) {
                Ok(directory) => Ok(directory),
                Err(error) => Err(export_error!(ExportErrorTy::FileStorageError(error)))
            }?;

            for _ in 0..max_count {
                let store = match lock(store_mutex) {
                    Ok(gaurd) => Ok(gaurd),
                    Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
                }?;
                let mut leveldb = match lock(database_mutex) {
                    Ok(gaurd) => Ok(gaurd),
                    Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
                }?;
                let mut file_storage = match lock(&file_storage_mutex) {
                    Ok(gaurd) => Ok(gaurd),
                    Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
                }?;
                let uuid = match store.get(None, None, false) {
                    Ok(uuid) => Ok(uuid),
                    Err(error) => Err(export_error!(ExportErrorTy::StoreError(error)))
                }?;
                let uuid = match uuid {
                    Some(uuid) => uuid,
                    None => { break }
                };
                let msg = match leveldb.get(uuid.clone()) {
                    Ok(msg) => Ok(msg),
                    Err(error) => Err(export_error!(ExportErrorTy::DatabaseError(error)))
                }?;
                let msg_byte_size = msg.len() as u64;

                let src_file_path = get_file_path_from_id(&file_storage.path, &uuid);
                let dest_file_path = get_file_path_from_id(&file_storage_export_directory, &uuid);
                if let Err(error) = copy(&src_file_path, &dest_file_path) {
                    return Err(export_error!(ExportErrorTy::CouldNotCopyFile, error));
                };
                // remove the file from the index
                if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                    return Err(export_error!(ExportErrorTy::FileStorageError(error)));
                }

                // add the data to the leveldb backup
                // if it errors then copy the destination file back to the source
                // dont exit until on error handling has finished
                if let Err(error) = leveldb_backup.add(uuid, msg, msg_byte_size) {
                    if let Err(error) = copy(&dest_file_path, &src_file_path) {
                        return Err(export_error!(ExportErrorTy::CouldNotReinsertFileAfterError, error));
                    };
                    if let Err(error) = remove_file(dest_file_path) {
                        return Err(export_error!(ExportErrorTy::CouldNotRemoveFileAfterError, error));
                    }
                    return Err(export_error!(ExportErrorTy::CouldNotAddFileToBackup(error)));
                }
                // update deleted count
                deleted_count += 1;    
            }
        } else {
            for _ in 0..max_count {
                let store = match lock(store_mutex) {
                    Ok(gaurd) => Ok(gaurd),
                    Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
                }?;
                let mut leveldb = match lock(database_mutex) {
                    Ok(gaurd) => Ok(gaurd),
                    Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
                }?;
                let uuid = match store.get(None, None, false) {
                    Ok(uuid) => Ok(uuid),
                    Err(error) => Err(export_error!(ExportErrorTy::StoreError(error)))
                }?;
                let uuid = match uuid {
                    Some(uuid) => uuid,
                    None => { break }
                };
                let msg = match leveldb.get(uuid.clone()) {
                    Ok(msg) => Ok(msg),
                    Err(error) => Err(export_error!(ExportErrorTy::DatabaseError(error)))
                }?;                
                let msg_byte_size = msg.len() as u64;

                // add the data to the leveldb backup
                // if it errors then copy the destination file back to the source
                // dont exit until on error handling has finished
                if let Err(error) = leveldb_backup.add(uuid, msg, msg_byte_size) {
                    return Err(export_error!(ExportErrorTy::DatabaseError(error)));
                }
                // update deleted count
                deleted_count += 1;    
            }
        }
        deleted_count
    };
    // update stats
    {
        let mut stats = match lock(stats_mutex) {
            Ok(gaurd) => Ok(gaurd),
            Err(error) => Err(export_error!(ExportErrorTy::LockError, error))
        }?;
        stats.deleted += deleted_count;
    }    
    Ok(())
}
