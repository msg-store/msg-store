use crate::api::error_codes::{self, log_err};
use crate::api::msg::add::Chunky;
use crate::core::uuid::Uuid;
use futures::StreamExt;
use std::collections::BTreeSet;
use std::fmt::Display;
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

#[derive(Debug)]
pub enum FileStorageErrorTy {
    CouldNotCreateDirectory,
    CouldNotCreateFile,
    CouldNotGetChunkFromPayload,
    CouldNotReadDirectory,
    CouldNotReadMetadata,
    CouldNotRemoveFile,
    CouldNotOpenFile,
    CouldNotParseChunk,
    CouldNotWriteToFIle,
    DirectoryDoesNotExist,
    PathIsNotADirectory
}
impl Display for FileStorageErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug)]
pub struct FileStorageError {
    pub err_ty: FileStorageErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for FileStorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FILE_STORAGE_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)?;
        if let Some(msg) = &self.msg {
            write!(f, "{}", msg)
        } else {
            Ok(())
        }
    }   
}

macro_rules! fs_error {
    ($err_ty:expr) => {
        FileStorageError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        FileStorageError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

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
pub fn create_directory(base_directory: &Path) -> Result<PathBuf, FileStorageError> {
    let mut file_storage_path = base_directory.to_path_buf();
    file_storage_path.push("file-storage");
    if let Err(error) = create_dir_all(&file_storage_path) {
        return Err(fs_error!(FileStorageErrorTy::CouldNotCreateDirectory, error))
    }
    Ok(file_storage_path)
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
pub fn read_file_storage_direcotory(file_storage_path: &Path) -> Result<Vec<Arc<Uuid>>, FileStorageError> {
    if !file_storage_path.exists() {
        return Err(fs_error!(FileStorageErrorTy::DirectoryDoesNotExist));
    }
    if !file_storage_path.is_dir() {
        return Err(fs_error!(FileStorageErrorTy::PathIsNotADirectory))
    }
    let uuids: Vec<Arc<Uuid>> = match read_dir(file_storage_path) {
        Ok(read_dir) => Ok(read_dir),
        Err(error) => Err(fs_error!(FileStorageErrorTy::CouldNotReadDirectory, error))
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

pub fn get_buffer(file_storage_path: &Path, uuid: &Uuid) -> Result<(BufReader<File>, u64), FileStorageError> {
    let uuid_string = uuid.to_string();
    let mut file_path = file_storage_path.to_path_buf();
    file_path.push(uuid_string);
    let file = match File::open(file_path) {
        Ok(file) => Ok(file),
        Err(error) => Err(fs_error!(FileStorageErrorTy::CouldNotOpenFile, error))
    }?;
    let metadata = match file.metadata() {
        Ok(metadata) => Ok(metadata),
        Err(error) => Err(fs_error!(FileStorageErrorTy::CouldNotReadMetadata, error))
    }?;
    let file_size = metadata.len();
    let buffer = BufReader::new(file);
    return Ok((buffer, file_size));
}

pub async fn write_to_disk<T: Chunky>(file_storage_path: &Path, uuid: &Uuid, first_chunk: &[u8], payload: &mut T) -> Result<(), FileStorageError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    let mut file = match File::create(file_path) {
        Ok(file) => Ok(file),
        Err(error) => Err(fs_error!(FileStorageErrorTy::CouldNotCreateFile, error))
    }?;
    if let Err(error) = file.write(first_chunk) {
        return Err(fs_error!(FileStorageErrorTy::CouldNotWriteToFIle, error))
    };
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => Ok(chunk),
            Err(error) => Err(fs_error!(FileStorageErrorTy::CouldNotGetChunkFromPayload, error))
        }?;
        if let Err(error) = file.write(&chunk) {
            return Err(fs_error!(FileStorageErrorTy::CouldNotWriteToFIle, error));
        };
    };
    Ok(())
}

pub fn rm_from_disk(file_storage_path: &Path, uuid: &Uuid) -> Result<bool, FileStorageError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if !file_path.exists() {
        return Ok(false)
    }
    if let Err(error) = rm_from_disk_wo_check(file_storage_path, uuid) {
        return Err(fs_error!(FileStorageErrorTy::CouldNotRemoveFile, error));
    };
    Ok(true)
}

pub fn rm_from_disk_wo_check(file_storage_path: &Path, uuid: &Uuid) -> Result<(), FileStorageError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if let Err(error) = remove_file(file_path) {
        return Err(fs_error!(FileStorageErrorTy::CouldNotRemoveFile, error));
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
pub fn rm_from_file_storage(file_storage: &mut FileStorage, uuid: &Uuid) -> Result<bool, FileStorageError> {
    if file_storage.index.remove(uuid) {
        rm_from_disk(&file_storage.path, uuid)
    } else {
        Ok(false)
    }
}

pub async fn add_to_file_storage<T: Chunky>(file_storage: &mut FileStorage, uuid: Arc<Uuid>, first_chunk: &[u8], payload: &mut T) -> Result<(), FileStorageError> {
    write_to_disk(&file_storage.path, &uuid, first_chunk, payload).await?;
    file_storage.index.insert(uuid.clone());
    Ok(())
}

pub fn discover_files(file_storage: &mut FileStorage, uuids: Vec<Arc<Uuid>>) {
    for uuid in uuids {
        file_storage.index.insert(uuid);
    }
}
