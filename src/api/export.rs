use crate::api::Database;
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
pub enum ErrTy {
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
impl Display for ErrTy {
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
pub struct ApiError {
    pub err_ty: ErrTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "EXPORT_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "EXPORT_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
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

fn create_export_directory(export_directory: &Path) -> Result<bool, ApiError> {
    if export_directory.exists() {
        if let Err(error) = create_dir_all(export_directory) {
            return Err(api_error!(ErrTy::CouldNotCreateDirectory, error))
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
) -> Result<(), ApiError> {

    let max_count = {
        let store = match store_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockError, err))
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

        // open the leveldb instance
        let mut leveldb_backup = match Leveldb::new(&leveldb_path) {
            Ok(leveldb) => Ok(leveldb),
            Err(error) => Err(api_error!(ErrTy::DatabaseError(error)))
        }?;

        if let Some(file_storage_mutex) = file_storage_option {

            // create file storage directory
            let file_storage_export_directory = match create_directory(&export_dir_path) {
                Ok(directory) => Ok(directory),
                Err(error) => Err(api_error!(ErrTy::FileStorageError(error)))
            }?;

            for _ in 0..max_count {
                let mut store = match store_mutex.lock() {
                    Ok(gaurd) => Ok(gaurd),
                    Err(err) => Err(api_error!(ErrTy::LockError, err))
                }?;
                let mut database = match database_mutex.lock() {
                    Ok(gaurd) => Ok(gaurd),
                    Err(err) => Err(api_error!(ErrTy::LockError, err))
                }?;
                let mut file_storage = match file_storage_mutex.lock() {
                    Ok(gaurd) => Ok(gaurd),
                    Err(err) => Err(api_error!(ErrTy::LockError, err))
                }?;
                let uuid = match store.get(None, None, false) {
                    Ok(uuid) => Ok(uuid),
                    Err(error) => Err(api_error!(ErrTy::StoreError(error)))
                }?;
                let uuid = match uuid {
                    Some(uuid) => uuid,
                    None => { break }
                };
                let msg = match database.get(uuid.clone()) {
                    Ok(msg) => Ok(msg),
                    Err(error) => Err(api_error!(ErrTy::DatabaseError(error)))
                }?;
                let msg_byte_size = msg.len() as u64;

                let src_file_path = get_file_path_from_id(&file_storage.path, &uuid);
                let dest_file_path = get_file_path_from_id(&file_storage_export_directory, &uuid);
                if let Err(error) = copy(&src_file_path, &dest_file_path) {
                    return Err(api_error!(ErrTy::CouldNotCopyFile, error));
                };
                // remove the file from the index
                if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                    return Err(api_error!(ErrTy::FileStorageError(error)));
                }

                // add the data to the leveldb backup
                // if it errors then copy the destination file back to the source
                // dont exit until on error handling has finished
                if let Err(error) = leveldb_backup.add(uuid.clone(), msg, msg_byte_size) {
                    if let Err(error) = copy(&dest_file_path, &src_file_path) {
                        return Err(api_error!(ErrTy::CouldNotReinsertFileAfterError, error));
                    };
                    if let Err(error) = remove_file(dest_file_path) {
                        return Err(api_error!(ErrTy::CouldNotRemoveFileAfterError, error));
                    }
                    return Err(api_error!(ErrTy::CouldNotAddFileToBackup(error)));
                }

                if let Err(err) = store.del(uuid.clone()) {
                    return Err(api_error!(ErrTy::StoreError(err)));
                }
                if let Err(err) = database.del(uuid.clone()) {
                    return Err(api_error!(ErrTy::DatabaseError(err)))
                }

                // update deleted count
                deleted_count += 1;    
            }
        } else {
            for _ in 0..max_count {
                let mut store = match store_mutex.lock() {
                    Ok(gaurd) => Ok(gaurd),
                    Err(err) => Err(api_error!(ErrTy::LockError, err))
                }?;
                let mut database = match database_mutex.lock() {
                    Ok(gaurd) => Ok(gaurd),
                    Err(err) => Err(api_error!(ErrTy::LockError, err))
                }?;
                let uuid = match store.get(None, None, false) {
                    Ok(uuid) => Ok(uuid),
                    Err(error) => Err(api_error!(ErrTy::StoreError(error)))
                }?;
                let uuid = match uuid {
                    Some(uuid) => uuid,
                    None => { break }
                };
                let msg = match database.get(uuid.clone()) {
                    Ok(msg) => Ok(msg),
                    Err(error) => Err(api_error!(ErrTy::DatabaseError(error)))
                }?;                
                let msg_byte_size = msg.len() as u64;

                // add the data to the leveldb backup
                // if it errors then copy the destination file back to the source
                // dont exit until on error handling has finished
                if let Err(error) = leveldb_backup.add(uuid.clone(), msg, msg_byte_size) {
                    return Err(api_error!(ErrTy::DatabaseError(error)));
                }

                if let Err(err) = store.del(uuid.clone()) {
                    return Err(api_error!(ErrTy::StoreError(err)));
                }
                if let Err(err) = database.del(uuid.clone()) {
                    return Err(api_error!(ErrTy::DatabaseError(err)))
                }

                // update deleted count
                deleted_count += 1;    
            }
        }
        deleted_count
    };
    // update stats
    {
        let mut stats = match stats_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(err) => Err(api_error!(ErrTy::LockError, err))
        }?;
        stats.deleted += deleted_count;
    }    
    Ok(())
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use crate::fake_payload;
    use crate::api::file_storage::FileStorage;
    use crate::api::msg::add::handle as add_handle;
    use crate::api::msg::tests::FakePayload;
    use crate::api::stats::Stats;
    use crate::core::store::Store;
    use crate::database::Db;
    use crate::database::leveldb::Leveldb;
    use futures::executor::block_on;
    use rand::prelude::random;
    use std::convert::AsRef;
    use std::fs::{read_to_string, remove_dir_all};
    use std::ops::Drop;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;
    
    use super::handle;
    
    use tempdir::TempDir;

    #[derive(Debug)]
    pub struct LazyTempDir {
        path: PathBuf
    }
    impl LazyTempDir {
        pub fn new(prefix: &str) -> LazyTempDir {
            let name: u128 = random();
            let name_string = format!("/tmp/{}-{}", prefix, name);
            LazyTempDir { path: PathBuf::from(name_string) }
        }
        pub fn path(&self) -> &Path {
            self.as_ref()
        }
    }
    impl AsRef<Path> for LazyTempDir {
        fn as_ref(&self) -> &Path {
            &self.path
        }
    }
    impl Drop for LazyTempDir {
        fn drop(&mut self) {
            if self.path.exists() {
                remove_dir_all(&self.path).unwrap()
            }
        }
    }

    #[test]
    fn should_export_file_based_msgs() {
        
        let tmp_dir = TempDir::new("should_export_msgs").unwrap();
        let tmp_export_dir = LazyTempDir::new("should_export_msgs_export");
        let level_db_path = {
            let mut level_db_path = tmp_dir.path().to_path_buf();
            level_db_path.push("leveldb");
            level_db_path
        };
        let file_storage_path = {
            let mut file_storage_path = tmp_dir.path().to_path_buf();
            file_storage_path.push("file-storage");
            file_storage_path
        };
        let exported_level_db_path = {
            let mut level_db_path = tmp_export_dir.path().to_path_buf();
            level_db_path.push("leveldb");
            level_db_path
        };
        let exported_file_storage_path = {
            let mut file_storage_path = tmp_export_dir.path().to_path_buf();
            file_storage_path.push("file-storage");
            file_storage_path
        };
        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(Leveldb::new(&level_db_path).unwrap()));
        let stats_mx = Mutex::new(Stats::new());
        
        
        let file_storage_op = Some(Mutex::new(FileStorage::new(&file_storage_path).unwrap()));

        // add a message to the store and database using the add msg api
        let msg = "Hello, world";
        let msg_len = msg.len() as u64;
        let payload_str = format!("priority=1&saveToFile=true&bytesizeOverride={}&fileName=my-file?{}", msg_len, msg);
        let payload = fake_payload!(payload_str);
        let uuid = block_on(add_handle(&store_mx, &file_storage_op, &stats_mx, &database_mx, payload)).unwrap();
        
        let msg_headers = {
            database_mx.lock().unwrap().get(uuid.clone()).unwrap()
        };

        handle(
            &store_mx, 
            &database_mx, 
            &file_storage_op, 
            &stats_mx, 
            tmp_export_dir.path()).unwrap();

        // make assertions
        {
            // the store should be empty
            let store = store_mx.lock().unwrap();
            assert!(store.byte_size == 0);
            assert!(store.id_to_group_map.len() == 0);
            
            // the database should be empty
            let mut database = database_mx.lock().unwrap();
            assert!(database.fetch().unwrap().len() == 0);

            // the stats object should have deleted 1
            let stats = stats_mx.lock().unwrap();
            assert!(stats.deleted == 1);

            // there should be no file in the original directory
            let file_path = {
                let mut file_path = file_storage_path.to_path_buf();
                file_path.push(uuid.to_string());
                file_path
            };
            assert!(!file_path.exists());

            // there should be a file in the output directory
            let file_path = {
                let mut file_path = exported_file_storage_path.to_path_buf();
                file_path.push(uuid.to_string());
                file_path
            };
            assert!(file_path.exists());

            // there should be a msg in the database
            let mut database = Leveldb::new(&exported_level_db_path).unwrap();
            assert!(database.fetch().unwrap().len() == 1);

            // the headers should match
            assert!(database.get(uuid.clone()).unwrap() == msg_headers);

            // the file contents should match
            assert!(read_to_string(file_path).unwrap() == msg);

        }

    }

    #[test]
    fn should_export_msgs() {
        
        let tmp_dir = TempDir::new("should_export_msgs").unwrap();
        let tmp_export_dir = LazyTempDir::new("should_export_msgs_export");
        let level_db_path = {
            let mut level_db_path = tmp_dir.path().to_path_buf();
            level_db_path.push("leveldb");
            level_db_path
        };

        let exported_level_db_path = {
            let mut level_db_path = tmp_export_dir.path().to_path_buf();
            level_db_path.push("leveldb");
            level_db_path
        };

        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(Leveldb::new(&level_db_path).unwrap()));
        let stats_mx = Mutex::new(Stats::new());
        
        
        // add a message to the store and database using the add msg api
        let msg = "Hello, world";
        let payload_str = format!("priority=1?{}", msg);
        let payload = fake_payload!(payload_str);
        let uuid = block_on(add_handle(&store_mx, &None, &stats_mx, &database_mx, payload)).unwrap();
        
        let inserted_msg = {
            database_mx.lock().unwrap().get(uuid.clone()).unwrap()
        };

        handle(
            &store_mx, 
            &database_mx, 
            &None, 
            &stats_mx, 
            tmp_export_dir.path()).unwrap();

        // make assertions
        {
            // the store should be empty
            let store = store_mx.lock().unwrap();
            assert!(store.byte_size == 0);
            assert!(store.id_to_group_map.len() == 0);
            
            // the database should be empty
            let mut database = database_mx.lock().unwrap();
            assert!(database.fetch().unwrap().len() == 0);

            // the stats object should have deleted 1
            let stats = stats_mx.lock().unwrap();
            assert!(stats.deleted == 1);

            // there should be a msg in the database
            let mut database = Leveldb::new(&exported_level_db_path).unwrap();
            assert!(database.fetch().unwrap().len() == 1);

            // the msg should match
            assert!(database.get(uuid.clone()).unwrap() == inserted_msg);
            assert!(database.get(uuid.clone()).unwrap() == msg);

        }

    }
}