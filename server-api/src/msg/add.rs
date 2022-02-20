use crate::Database;
use crate::file_storage::{
    rm_from_file_storage,
    add_to_file_storage,
    FileStorage
};
use crate::stats::Stats;
use crate::file_storage::FileStorageError;
use msg_store::{Store, StoreErrorTy};
use msg_store_database_plugin::DatabaseError;
use msg_store_uuid::Uuid;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use std::collections::BTreeMap;
use std::fmt::Display;
use std::marker::Unpin;
use std::sync::{Arc,Mutex};

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
        match self {
            Self::FileStorageNotConfigured |
            Self::InvalidBytesizeOverride |
            Self::InvalidPriority |
            Self::MissingBytesizeOverride |
            Self::MissingHeaders |
            Self::MissingPriority |
            Self::MalformedHeaders |
            Self::MsgExceedesGroupMax |
            Self::MsgExceedesStoreMax |
            Self::MsgLacksPriority |
            Self::CouldNotGetNextChunkFromPayload |
            Self::CouldNotParseChunk => write!(f, "MSG_ERROR: {:#?}", self)
        }
    }
}

#[derive(Debug)]
pub enum AddErrorTy {
    DatabaseError(DatabaseError),
    FileStorageError(FileStorageError),
    MsgError(MsgError),
    StoreError(StoreErrorTy),
    CouldNotFindFileStorage,
    LockingError
}
impl Display for AddErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DatabaseError(err) => write!(f, "({})", err),
            Self::FileStorageError(err) => write!(f, "({})", err),
            Self::MsgError(err) => write!(f, "({})", err),
            Self::StoreError(err) => write!(f, "({})", err),
            Self::CouldNotFindFileStorage |
            Self::LockingError => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct AddError {
    pub err_ty: AddErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for AddError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "ADD_MSG_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "ADD_MSG_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! add_msg_error {
    ($err_ty:expr) => {
        AddError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        AddError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub trait Chunky: Stream<Item=Result<Bytes, &'static str>> + Unpin { }

pub async fn handle<T: Chunky>(
    store: &Mutex<Store>,
    file_storage: &Option<Mutex<FileStorage>>,
    stats: &Mutex<Stats>,
    database: &Mutex<Database>,
    mut payload: T) -> Result<Arc<Uuid>, AddError> {

        let mut metadata_string = String::new();
        let mut msg_chunk = BytesMut::new();
        let mut metadata: BTreeMap<String, String> = BTreeMap::new();
        let mut save_to_file = false;
    
        while let Some(chunk) = payload.next().await {
            let chunk = match chunk {
                Ok(chunk) => Ok(chunk),
                Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::CouldNotGetNextChunkFromPayload), error))
            }?;
            let mut chunk_string = match String::from_utf8(chunk.to_vec()) {
                Ok(chunk_string) => Ok(chunk_string),
                Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::CouldNotParseChunk), error))
            }?;
            chunk_string = chunk_string.trim_start().to_string();
            // debug!("recieved chunk: {}", chunk_string);
            metadata_string.push_str(&chunk_string);
            if metadata_string.contains("?") {
                // debug!("msg start found!");
                if metadata_string.len() == 0 {
                    return Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MissingHeaders)));
                }
                match metadata_string.split_once("?") {
                    Some((metadata_section, msg_section)) => {
                        for pair in metadata_section.to_string().split("&").into_iter() {
                            let kv = pair.trim_end().trim_start().split("=").map(|txt| txt.to_string()).collect::<Vec<String>>();
                            let k = match kv.get(0) {
                                Some(k) => Ok(k.clone()),
                                None => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MalformedHeaders)))
                            }?;
                            let v = match kv.get(1) {
                                Some(v) => Ok(v.clone()),
                                None => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MalformedHeaders)))
                            }?;
                            metadata.insert(k, v);
                        };
                        msg_chunk.extend_from_slice(msg_section.as_bytes());
                    },
                    None => {
                        return Err(add_msg_error!(AddErrorTy::MsgError(MsgError::CouldNotParseChunk)))
                    }
                }
                if let Some(save_to_file_value) = metadata.get("saveToFile") {
                    if save_to_file_value.to_lowercase() == "true" {
                        if let None = file_storage {
                            while let Some(_chunk) = payload.next().await {
    
                            }
                            return Err(add_msg_error!(AddErrorTy::MsgError(MsgError::FileStorageNotConfigured)));
                        }
                        save_to_file = true;
                    }
                }
                break;
            }
        }
    
        let priority: u32 = match metadata.remove("priority") {
            Some(priority) => match priority.parse::<u32>() {
                Ok(priority) => Ok(priority),
                Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::InvalidPriority), error))
            },
            None => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MissingPriority)))
        }?;

        let (msg_byte_size, msg) = {
            if save_to_file == true {
                if let Some(byte_size_override_str) = metadata.get("bytesizeOverride") {
                    let msg_byte_size = match byte_size_override_str.parse::<u64>() {
                        Ok(byte_size_override) => Ok(byte_size_override),
                        Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::InvalidBytesizeOverride), error))
                    }?;
                    let msg_parse = metadata
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<String>>()
                        .join("&");
                    Ok((msg_byte_size, msg_parse))
                } else {
                    Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MissingBytesizeOverride)))
                }
            } else {
                while let Some(chunk) = payload.next().await {
                    let chunk = match chunk {
                        Ok(chunk) => Ok(chunk),
                        Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::CouldNotGetNextChunkFromPayload), error))
                    }?;
                    msg_chunk.extend_from_slice(&chunk);
                }
                match String::from_utf8(msg_chunk.to_vec()) {
                    Ok(msg) => Ok((msg.len() as u64, msg)),
                    Err(error) => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::CouldNotParseChunk), error))
                }
            }
        }?;
        let add_result = {
            let mut store = match store.lock() {
                Ok(store) => Ok(store),
                Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
            }?;
            match store.add(priority, msg_byte_size) {
                Ok(add_result) => Ok(add_result),
                Err(error) => match error.err_ty {
                    StoreErrorTy::ExceedesStoreMax => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MsgExceedesStoreMax))),
                    StoreErrorTy::ExceedesGroupMax => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MsgExceedesGroupMax))),
                    StoreErrorTy::LacksPriority => Err(add_msg_error!(AddErrorTy::MsgError(MsgError::MsgLacksPriority))),
                    error_ty => Err(add_msg_error!(AddErrorTy::StoreError(error_ty)))
                },
            }
        }?;
        
        // remove msgs from db
        let mut deleted_count = 0;
        for uuid in add_result.msgs_removed.into_iter() {
            {
                let mut database = match database.lock() {
                    Ok(database) => Ok(database),
                    Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
                }?;
                if let Err(error) = database.del(uuid.clone()) {
                    return Err(add_msg_error!(AddErrorTy::DatabaseError(error)))
                }
            }
            if let Some(file_storage) = &file_storage {
                let mut file_storage = match file_storage.lock() {
                    Ok(file_storage) => Ok(file_storage),
                    Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
                }?;
                if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                    return Err(add_msg_error!(AddErrorTy::FileStorageError(error)));
                }
            }
            deleted_count += 1;
        }
        {
            let mut stats = match stats.lock() {
                Ok(stats) => Ok(stats),
                Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
            }?;
            stats.pruned += deleted_count;
            stats.inserted += 1;
        }
        // add to file manager if needed
        if save_to_file {
            if let Some(file_storage) = file_storage {
                let mut file_storage = match file_storage.lock() {
                    Ok(file_storage) => Ok(file_storage),
                    Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
                }?;
                if let Err(error) = add_to_file_storage(&mut file_storage, add_result.uuid.clone(), &msg_chunk, payload).await {
                    return Err(add_msg_error!(AddErrorTy::FileStorageError(error)))
                }
            } else {
                return Err(add_msg_error!(AddErrorTy::CouldNotFindFileStorage));
            }
        }
        {        
            let mut database = match database.lock() {
                Ok(database) => Ok(database),
                Err(error) => Err(add_msg_error!(AddErrorTy::LockingError, error))
            }?;
            if let Err(error) = database.add(add_result.uuid.clone(), Bytes::copy_from_slice(msg.as_bytes()), msg_byte_size) {
                return Err(add_msg_error!(AddErrorTy::DatabaseError(error)));
            }
        }
        Ok(add_result.uuid)
}
