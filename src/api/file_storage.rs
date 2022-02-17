use bytes::Bytes;
use crate::api::msg::add::Chunky;
use crate::core::uuid::Uuid;
use futures::{Stream, StreamExt, task::Context};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::Display;
use std::fs::{
    create_dir_all,
    read_dir,
    remove_file,
    File,
};
use std::io::{
    BufReader,
    Read,
    Write
};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

#[derive(Debug, PartialEq, Eq)]
pub enum FsErrType {
    CouldNotCreateDirectory,
    CouldNotCreateFile,
    CouldNotGetMetadata,
    CouldNotGetChunkFromPayload,
    CouldNotOpenFile,
    CouldNotReadDirectory,
    CouldNotReadFile,
    CouldNotRevoveFile,
    CouldNotWriteToFile,
    DirectoryDoesNotExist,
    PathIsNotADirectory
}
impl Display for FsErrType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FsError {
    pub err_type: FsErrType,
    pub msg: Option<String>,
    pub file: &'static str,
    pub line: u32
}
impl FsError {
    pub fn new(err_type: FsErrType, file: &'static str, line: u32) -> FsError {
        FsError { err_type, file, line, msg: None }
    }
    pub fn new_with_msg(err_type: FsErrType, file: &'static str, line: u32, error: String) -> FsError {
        FsError { err_type, file, line, msg: Some(error) }
    }
}
impl Display for FsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self)
    }
}
impl Error for FsError {

}

macro_rules! fs_error {
    ($fs_error_type:expr) => {
        FsError::new($fs_error_type, file!(), line!())
    };
    ($fs_error_type:expr, $error_msg:expr) => {
        FsError::new_with_msg($fs_error_type, file!(), line!(), $error_msg.to_string())
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
pub fn create_directory(base_directory: &Path) -> Result<PathBuf, FsError> {
    let mut file_storage_path = base_directory.to_path_buf();
    file_storage_path.push("file-storage");
    match create_dir_all(&file_storage_path) {
        Ok(_) => Ok(file_storage_path),
        Err(error) => Err(fs_error!(FsErrType::CouldNotCreateDirectory, error))
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
pub fn read_file_storage_direcotory(file_storage_path: &Path) -> Result<Vec<Arc<Uuid>>, FsError> {
    if !file_storage_path.exists() {
        return Err(fs_error!(FsErrType::DirectoryDoesNotExist));
    }
    if !file_storage_path.is_dir() {
        return Err(fs_error!(FsErrType::PathIsNotADirectory))
    }
    let uuids: Vec<Arc<Uuid>> = match read_dir(file_storage_path) {
        Ok(read_dir) => Ok(read_dir),
        Err(error) => Err(fs_error!(FsErrType::CouldNotReadDirectory, error))
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

pub fn get_buffer(file_storage_path: &Path, uuid: &Uuid) -> Result<(BufReader<File>, u64), FsError> {
    let uuid_string = uuid.to_string();
    let mut file_path = file_storage_path.to_path_buf();
    file_path.push(uuid_string);
    let file = match File::open(file_path) {
        Ok(file) => Ok(file),
        Err(error) => Err(fs_error!(FsErrType::CouldNotOpenFile, error))
    }?;
    let metadata = match file.metadata() {
        Ok(metadata) => Ok(metadata),
        Err(error) => Err(fs_error!(FsErrType::CouldNotGetMetadata, error))
    }?;
    let file_size = metadata.len();
    let buffer = BufReader::new(file);
    return Ok((buffer, file_size));
}

pub async fn write_to_disk<E: Error, T: Chunky<E>>(file_storage_path: &Path, uuid: &Uuid, first_chunk: &[u8], payload: &mut T) -> Result<(), FsError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    let mut file = match File::create(file_path) {
        Ok(file) => Ok(file),
        Err(error) => Err(fs_error!(FsErrType::CouldNotCreateFile, error))
    }?;
    if let Err(error) = file.write(first_chunk) {
        return Err(fs_error!(FsErrType::CouldNotWriteToFile, error));
    };
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(chunk) => Ok(chunk),
            Err(error) => Err(fs_error!(FsErrType::CouldNotGetChunkFromPayload, error))
        }?;
        if let Err(error) = file.write(&chunk) {
            return Err(fs_error!(FsErrType::CouldNotWriteToFile, error));
        };
    };
    Ok(())
}

pub fn rm_from_disk(file_storage_path: &Path, uuid: &Uuid) -> Result<bool, FsError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if !file_path.exists() {
        return Ok(false)
    }
    if let Err(error) = rm_from_disk_wo_check(file_storage_path, uuid) {
        return Err(fs_error!(FsErrType::CouldNotRevoveFile, error));
    };
    Ok(true)
}

pub fn rm_from_disk_wo_check(file_storage_path: &Path, uuid: &Uuid) -> Result<(), FsError> {
    let file_path = get_file_path_from_id(file_storage_path, uuid);
    if let Err(error) = remove_file(file_path) {
        return Err(fs_error!(FsErrType::CouldNotRevoveFile, error));
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
pub fn rm_from_file_storage(file_storage: &mut FileStorage, uuid: &Uuid) -> Result<bool, FsError> {
    if file_storage.index.remove(uuid) {
        rm_from_disk(&file_storage.path, uuid)
    } else {
        Ok(false)
    }
}

pub async fn add_to_file_storage<E: Error, T: Chunky<E>>(file_storage: &mut FileStorage, uuid: Arc<Uuid>, first_chunk: &[u8], payload: &mut T) -> Result<(), FsError> {
    write_to_disk(&file_storage.path, &uuid, first_chunk, payload).await?;
    file_storage.index.insert(uuid.clone());
    Ok(())
}

pub fn discover_files(file_storage: &mut FileStorage, uuids: Vec<Arc<Uuid>>) {
    for uuid in uuids {
        file_storage.index.insert(uuid);
    }
}

pub struct ReturnBody {
    pub header: String,
    pub msg: BufReader<File>,
    pub file_size: u64,
    pub bytes_read: u64,
    pub headers_sent: bool,
    pub msg_sent: bool
}
impl ReturnBody {
    pub fn new(header: String, file_size: u64, msg: BufReader<File>) -> ReturnBody {
        ReturnBody {
            header,
            file_size,
            bytes_read: 0,
            msg,
            headers_sent: false,
            msg_sent: false
        }
    }
}
impl Stream for ReturnBody {
    type Item = Result<Bytes, FsError>;
    fn poll_next(
        mut self: Pin<&mut Self>, 
        _cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        if self.msg_sent {
            return Poll::Ready(None);
        }
        if self.headers_sent {            
            let limit = self.file_size - self.bytes_read;
            if limit >= 665600 {
                let mut buffer = [0; 665600];
                if let Err(error) = self.msg.read(&mut buffer) {
                    return Poll::Ready(Some(Err(fs_error!(FsErrType::CouldNotReadFile, error))));
                }
                {
                    let mut body = self.as_mut().get_mut();
                    body.bytes_read += 665600;
                }
                return Poll::Ready(Some(Ok(Bytes::copy_from_slice(&buffer))));
            } else if limit == 0 {
                return Poll::Ready(None);
            } else {
                let mut buffer = Vec::with_capacity(limit as usize);
                if let Err(error) = self.msg.read_to_end(&mut buffer) {
                    return Poll::Ready(Some(Err(fs_error!(FsErrType::CouldNotReadFile, error))));
                };
                {
                    let mut body = self.as_mut().get_mut();
                    body.msg_sent = true;
                }
                return Poll::Ready(Some(Ok(Bytes::copy_from_slice(&buffer))));
            }
        } else {
            {
                let mut body = self.as_mut().get_mut();
                body.headers_sent = true;
            }
            Poll::Ready(Some(Ok(Bytes::copy_from_slice(&self.header.as_bytes()))))
        }
    }
}