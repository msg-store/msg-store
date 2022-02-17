use crate::api::{Database, lock, ApiError, ApiErrorTy, MsgError};
use crate::api::file_storage::{
    rm_from_file_storage,
    add_to_file_storage,
    FileStorage
};
use crate::api::stats::Stats;
use crate::core::store::{Store, StoreErrorTy};
use crate::core::uuid::Uuid;
use crate::api_err;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::marker::Unpin;
use std::sync::{Arc,Mutex};

#[derive(Debug, Deserialize, Serialize)]
pub struct Body {
    priority: u32,
    msg: String,
}

pub trait Chunky<E: Error>: Stream<Item=Result<Bytes, E>> + Unpin { }

pub async fn try_add<P: Error, T: Chunky<P>, E: Error>(
    store: &Mutex<Store>,
    file_storage: &Option<Mutex<FileStorage>>,
    stats: &Mutex<Stats>,
    database: &Mutex<Database<E>>,
    payload: &mut T) -> Result<Arc<Uuid>, ApiError<E, P>> {

        let mut metadata_string = String::new();
        let mut msg_chunk = BytesMut::new();
        let mut metadata: BTreeMap<String, String> = BTreeMap::new();
        let mut save_to_file = false;
    
        while let Some(chunk) = payload.next().await {
            let chunk = match chunk {
                Ok(chunk) => Ok(chunk),
                Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
            }?;
            let mut chunk_string = match String::from_utf8(chunk.to_vec()) {
                Ok(chunk_string) => Ok(chunk_string),
                Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
            }?;
            chunk_string = chunk_string.trim_start().to_string();
            // debug!("recieved chunk: {}", chunk_string);
            metadata_string.push_str(&chunk_string);
            if metadata_string.contains("?") {
                // debug!("msg start found!");
                if metadata_string.len() == 0 {
                    return Err(api_err!(ApiErrorTy::MsgError(MsgError::MissingHeaders)));
                }
                match metadata_string.split_once("?") {
                    Some((metadata_section, msg_section)) => {
                        for pair in metadata_section.to_string().split("&").into_iter() {
                            let kv = pair.trim_end().trim_start().split("=").map(|txt| txt.to_string()).collect::<Vec<String>>();
                            let k = match kv.get(0) {
                                Some(k) => Ok(k.clone()),
                                None => Err(api_err!(ApiErrorTy::MsgError(MsgError::InvalidHeaders)))
                            }?;
                            let v = match kv.get(1) {
                                Some(v) => Ok(v.clone()),
                                None => Err(api_err!(ApiErrorTy::MsgError(MsgError::InvalidHeaders)))
                            }?;
                            metadata.insert(k, v);
                        };
                        msg_chunk.extend_from_slice(msg_section.as_bytes());
                    },
                    None => {
                        return Err(api_err!(ApiErrorTy::CouldNotParseChunk))
                    }
                }
                if let Some(save_to_file_value) = metadata.remove("saveToFile") {
                    if save_to_file_value.to_lowercase() == "true" {
                        if let None = file_storage {
                            while let Some(_chunk) = payload.next().await {
    
                            }
                            return Err(api_err!(ApiErrorTy::MsgError(MsgError::FileStorageNotConfigured)));
                        }
                        save_to_file = true;
                    }
                }
                break;
            }
        }
    
        let priority: u32 = match metadata.remove("priority") {
            Some(priority) => match priority.parse() {
                Ok(priority) => Ok(priority),
                Err(_error) => Err(api_err!(ApiErrorTy::MsgError(MsgError::InvalidPriority)))
            },
            None => Err(api_err!(ApiErrorTy::MsgError(MsgError::MissingPriority)))
        }?;

        let (msg_byte_size, msg) = {
            if save_to_file == true {
                if let Some(byte_size_override_str) = metadata.get("byteSizeOverride") {
                    let msg_byte_size = match byte_size_override_str.parse::<u32>() {
                        Ok(byte_size_override) => Ok(byte_size_override),
                        Err(_error) => Err(api_err!(ApiErrorTy::MsgError(MsgError::InvalidPriority)))
                    }?;
                    let msg_parse = metadata
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<String>>()
                        .join("&");
                    Ok((msg_byte_size, msg_parse))
                } else {
                    Err(api_err!(ApiErrorTy::MsgError(MsgError::MissingByteSizeOverride)))
                }
            } else {
                while let Some(chunk) = payload.next().await {
                    let chunk = match chunk {
                        Ok(chunk) => Ok(chunk),
                        Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
                    }?;
                    msg_chunk.extend_from_slice(&chunk);
                }
                match String::from_utf8(msg_chunk.to_vec()) {
                    Ok(msg) => Ok((msg.len() as u32, msg)),
                    Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
                }
            }
        }?;
        let add_result = {
            let mut store = lock(&store)?;
            match store.add(priority, msg_byte_size) {
                Ok(add_result) => Ok(add_result),
                Err(error) => match &error.err_ty {
                    StoreErrorTy::ExceedesStoreMax => Err(api_err!(ApiErrorTy::MsgError(MsgError::MsgExceedesStoreMax))),
                    StoreErrorTy::ExceedesGroupMax => Err(api_err!(ApiErrorTy::MsgError(MsgError::MsgExceedsGroupMax))),
                    StoreErrorTy::LacksPriority => Err(api_err!(ApiErrorTy::MsgError(MsgError::MsgLacksPriority))),
                    _error_ty => Err(api_err!(ApiErrorTy::StoreError(error)))
                },
            }
        }?;
        
        // remove msgs from db
        let mut deleted_count = 0;
        for uuid in add_result.msgs_removed.into_iter() {
            {
                let mut database = lock(&database)?;
                if let Err(error) = database.del(uuid.clone()) {
                    return Err(api_err!(ApiErrorTy::DbError(error)))
                }
            }
            if let Some(file_storage) = &file_storage {
                let mut file_storage = lock(&file_storage)?;
                if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                    return Err(api_err!(ApiErrorTy::FileStorageError(error)));
                }
            }
            deleted_count += 1;
        }
        {
            let mut stats = lock(&stats)?;
            stats.deleted += deleted_count;
            stats.inserted += 1;
        }
        // add to file manager if needed
        if save_to_file {
            if let Some(file_storage) = file_storage {
                let mut file_storage = lock(&file_storage)?;
                if let Err(error) = add_to_file_storage(&mut file_storage, add_result.uuid.clone(), &msg_chunk, payload).await {
                    return Err(api_err!(ApiErrorTy::FileStorageError(error)))
                }
            } else {
                return Err(api_err!(ApiErrorTy::FileStorageNotFound));
            }
        }
        {        
            let mut database = lock(&database)?;
            if let Err(error) = database.add(add_result.uuid.clone(), Bytes::copy_from_slice(msg.as_bytes()), msg_byte_size) {
                // log_err(error_codes::DATABASE_ERROR, file!(), line!(), error);
                return Err(api_err!(ApiErrorTy::DbError(error)));
            }
        }
        Ok(add_result.uuid)
}
