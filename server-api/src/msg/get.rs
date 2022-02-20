use bytes::Bytes;
use crate::{Database, Either};
use crate::file_storage::{get_buffer, FileStorage, FileStorageError};
use msg_store::{Store, StoreError};
use msg_store_uuid::Uuid;
use database_plugin::DatabaseError;
use futures::stream::Stream;
use futures::task::{Context, Poll};
use std::fmt::Display;
use std::fs::File;
use std::pin::Pin;
use std::io::{BufReader, Read};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum GetErrorTy {
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    MsgError(MsgError),
    StoreError(StoreError),
    CouldNotFindFileStorage,
    LockingError,
    CouldNotGetNextChunkFromPayload,
    CouldNotParseChunk
}
impl Display for GetErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::MsgError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::CouldNotFindFileStorage |
            Self::LockingError |
            Self::CouldNotGetNextChunkFromPayload |
            Self::CouldNotParseChunk => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct GetError {
    pub err_ty: GetErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "GET_MSG_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "GET_MSG_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! get_msg_error {
    ($err_ty:expr) => {
        GetError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        GetError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
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
    type Item = Result<Bytes, GetError>;
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
                    return Poll::Ready(Some(Err(get_msg_error!(GetErrorTy::CouldNotGetNextChunkFromPayload, error))));
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
                    return Poll::Ready(Some(Err(get_msg_error!(GetErrorTy::CouldNotParseChunk, error))));
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

#[derive(Debug, PartialEq, Eq)]
pub enum MsgError {
    FileStorageNotConfigured,
    InvalidBytesizeOverride,
    InvalidPriority,
    MissingBytesizeOverride,
    MissingHeaders,
    MissingPriority,
    MalformedHeaders,
    MsgExceedesGroupMax,
    MsgExceedesStoreMax,
    MsgLacksPriority,
    CouldNotGetNextChunkFromPayload,
    CouldNotParseChunk
}
impl Display for MsgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}


pub fn handle(
    store: &Mutex<Store>,
    database_mutex: &Mutex<Database>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    uuid_option: Option<Arc<Uuid>>,
    priority_option: Option<u32>,
    reverse_option: bool
) -> Result<Option<Either<ReturnBody, String>>, GetError> {
    let uuid = {
        let store = match store.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(error) => Err(get_msg_error!(GetErrorTy::LockingError, error))
        }?;
        match store.get(uuid_option, priority_option, reverse_option) {
            Ok(uuid) => match uuid {
                Some(uuid) => Ok(uuid),
                None => return Ok(None)
            },
            Err(error) => Err(get_msg_error!(GetErrorTy::StoreError(error)))
        }
    }?;
    let msg = {
        let mut database = match database_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(error) => Err(get_msg_error!(GetErrorTy::LockingError, error))
        }?;
        match database.get(uuid.clone()) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(get_msg_error!(GetErrorTy::DatabaseError(error)))
        }
    }?;
    if let Some(file_storage_mutex) = &file_storage_option {
        let file_storage = match file_storage_mutex.lock() {
            Ok(gaurd) => Ok(gaurd),
            Err(error) => Err(get_msg_error!(GetErrorTy::LockingError, error))
        }?;
        if file_storage.index.contains(&uuid) {
            let (file_buffer, file_size) = match get_buffer(&file_storage.path, &uuid) {
                Ok(buffer_option) => Ok(buffer_option),
                Err(error) => Err(get_msg_error!(GetErrorTy::FileStorageError(error)))
            }?;
            let msg_header = match String::from_utf8(msg.to_vec()) {
                Ok(msg_header) => Ok(msg_header),
                Err(error) => Err(get_msg_error!(GetErrorTy::CouldNotParseChunk, error))
            }?;
            let body = ReturnBody::new(format!("uuid={}&{}?", uuid.to_string(), msg_header), file_size, file_buffer);
            Ok(Some(Either::A(body)))
        } else {
            let msg = match String::from_utf8(msg.to_vec()) {
                Ok(msg) => Ok(msg),
                Err(error) => Err(get_msg_error!(GetErrorTy::CouldNotParseChunk, error))
            }?;
            Ok(Some(Either::B(format!("uuid={}?{}", uuid.to_string(), msg))))
        }
    } else {
        let msg = match String::from_utf8(msg.to_vec()) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(get_msg_error!(GetErrorTy::CouldNotParseChunk, error))
        }?;
        Ok(Some(Either::B(format!("uuid={}?{}", uuid.to_string(), msg))))
    }
}
