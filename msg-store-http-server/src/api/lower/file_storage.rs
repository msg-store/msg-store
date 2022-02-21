use crate::api::lower::error_codes::{self, log_err};
use crate::api::lower::msg::add::Chunky;
use futures::StreamExt;
use msg_store::Uuid;
use std::collections::BTreeSet;
use std::fs::{
    create_dir_all,
    read_dir,
    remove_file,
    File,
};
use std::io::{
    BufReader,
    Write
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct FileStorage {
    pub index: BTreeSet<Arc<Uuid>>,
    pub path: PathBuf
}
impl FileStorage {
    pub fn new(storage_path: &Path) -> FileStorage {
        FileStorage {
            index: BTreeSet::new(),
            path: storage_path.to_path_buf()
        }
    }
}

/// create a new dectory if one does not exist
/// returns Ok(false) if no directory was created because it already existed
pub fn create_directory(base_directory: &Path) -> Result<PathBuf, &'static str> {
    let mut file_storage_path = base_directory.to_path_buf();
    file_storage_path.push("file-storage");
    match create_dir_all(&file_storage_path) {
        Ok(_) => Ok(file_storage_path),
        Err(error) => {
            Err(error_codes::could_not_create_directory(file!(), line!(), error.to_string()))
        }
    }
}

/// Create a the file path for a file from the file storage path
pub fn get_file_path_from_id(file_storage_path: &Path, uuid: &Uuid) -> PathBuf {
    let mut file_path = file_storage_path.to_path_buf();
    file_path.push(uuid.to_string());
    file_path
}

/// reads the contents of a directory, returning a vec of uuids that it finds
/// 
/// ## Errors:
/// * If the the directory is not found
/// * If the path is not a directory
/// 
/// ## Notes
/// * Will ignore files that contain errors while reading, getting metadata or converting file
/// name to uuid
pub fn read_file_storage_direcotory(file_storage_path: &Path) -> Result<Vec<Arc<Uuid>>, &'static str> {
    if !file_storage_path.exists() {
        log_err(error_codes::DIRECTORY_DOES_NOT_EXIST, file!(), line!(), "Directory does not exist.");
        return Err(error_codes::DIRECTORY_DOES_NOT_EXIST);
    }
    if !file_storage_path.is_dir() {
        log_err(error_codes::PATH_IS_NOT_A_DIRECTORY, file!(), line!(), "Path is not a directory.");
        return Err(error_codes::PATH_IS_NOT_A_DIRECTORY)
    }
    let uuids: Vec<Arc<Uuid>> = match read_dir(file_storage_path) {
        Ok(read_dir) => Ok(read_dir),
        Err(error) => {
            log_err(error_codes::COULD_NOT_READ_DIRECTORY, file!(), line!(), error.to_string());
            Err(error_codes::COULD_NOT_READ_DIRECTORY)
        }
    }?.filter_map(|entry| {
        entry.ok()
    }).filter_map(|entry| {
        if let Ok(metadata) = entry.metadata() {
            Some((entry, metadata.is_file()))
        } else {
            None
        }
    }).filter_map(|(entry, is_file)| {
        if is_file {
            Some(entry)
        } else {
            None
        }
    }).filter_map(|entry| {
        if let Some(file_name_string) = entry.file_name().to_str() {
            Some(file_name_string.to_string())
        } else {
            None
        }         
    }).filter_map(|file_name| {
        if let Ok(uuid) = Uuid::from_string(&file_name) {
            Some(uuid)
        } else {
            None
        }
    }).collect();
    Ok(uuids)
}

pub fn get_buffer(file_storage_path: &Path, uuid: &Uuid) -> Result<(BufReader<File>, u64), &'static str> {
    let uuid_string = uuid.to_string();
    let mut file_path = file_storage_path.to_path_buf();
    file_path.push(uuid_string);
    let file = match File::open(file_path) {
        Ok(file) => Ok(file),
        Err(error) => {
            log_err(error_codes::COULD_NOT_OPEN_FILE, file!(), line!(), error.to_string());
            Err(error_codes::COULD_NOT_OPEN_FILE)
        }
    }?;
    let metadata = match file.metadata() {
        Ok(metadata) => Ok(metadata),
        Err(error) => {
            log_err(error_codes::COULD_NOT_GET_METADATA, file!(), line!(), error.to_string());
            Err(error_codes::COULD_NOT_GET_METADATA)
        }
    }?;
    let file_size = metadata.len();
    let buffer = BufReader::new(file);
    return Ok((buffer, file_size));
}

pub async fn write_to_disk<T: Chunky>(file_storage_path: &Path, uuid: &Uuid, first_chunk: &[u8], payload: &mut T) -> Result<(), &'static str> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    let mut file = match File::create(file_path) {
        Ok(file) => Ok(file),
        Err(error) => {
            log_err(error_codes::COULD_NOT_CREATE_FILE, file!(), line!(), error.to_string());
            Err(error_codes::COULD_NOT_CREATE_FILE)
        }
    }?;
    if let Err(error) = file.write(first_chunk) {
        log_err(error_codes::COULD_NOT_WRITE_TO_FILE, file!(), line!(), error.to_string());
        return Err(error_codes::COULD_NOT_WRITE_TO_FILE);
    };
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => Ok(chunk),
            Err(error) => {
                log_err(error_codes::COULD_NOT_GET_CHUNK_FROM_PAYLOAD, file!(), line!(), error.to_string());
                Err(error_codes::COULD_NOT_GET_CHUNK_FROM_PAYLOAD)
            }
        }?;
        if let Err(error) = file.write(&chunk) {
            log_err(error_codes::COULD_NOT_WRITE_TO_FILE, file!(), line!(), error.to_string());
            return Err(error_codes::COULD_NOT_WRITE_TO_FILE);
        };
    };
    Ok(())
}

pub fn rm_from_disk(file_storage_path: &Path, uuid: &Uuid) -> Result<bool, &'static str> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if !file_path.exists() {
        return Ok(false)
    }
    if let Err(error) = rm_from_disk_wo_check(file_storage_path, uuid) {
        log_err(error_codes::COULD_NOT_REMOVE_FILE, file!(), line!(), error.to_string());
        return Err(error_codes::COULD_NOT_REMOVE_FILE);
    };
    Ok(true)
}

pub fn rm_from_disk_wo_check(file_storage_path: &Path, uuid: &Uuid) -> Result<(), &'static str> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if let Err(error) = remove_file(file_path) {
        log_err(error_codes::COULD_NOT_REMOVE_FILE, file!(), line!(), error.to_string());
        return Err(error_codes::COULD_NOT_REMOVE_FILE);
    }
    Ok(())
}

/// Removes a file from the list and from disk
///
/// ## ERROR:
/// * If the file could not be removed
/// 
/// ## Returns
/// Ok(true) if the file was removed
/// Ok(false) if the file was already removed
pub fn rm_from_file_storage(file_storage: &mut FileStorage, uuid: &Uuid) -> Result<bool, &'static str> {
    if file_storage.index.remove(uuid) {
        rm_from_disk(&file_storage.path, uuid)
    } else {
        Ok(false)
    }
}

pub async fn add_to_file_storage<T: Chunky>(file_storage: &mut FileStorage, uuid: Arc<Uuid>, first_chunk: &[u8], payload: &mut T) -> Result<(), &'static str> {
    write_to_disk(&file_storage.path, &uuid, first_chunk, payload).await?;
    file_storage.index.insert(uuid.clone());
    Ok(())
}

pub fn discover_files(file_storage: &mut FileStorage, uuids: Vec<Arc<Uuid>>) {
    for uuid in uuids {
        file_storage.index.insert(uuid);
    }
}
