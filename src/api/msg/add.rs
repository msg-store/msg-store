use crate::api::{Database, lock};
use crate::api::error_codes::{ self, log_err };
use crate::api::file_storage::{
    rm_from_file_storage,
    add_to_file_storage,
    FileStorage
};
use crate::api::stats::Stats;
use crate::core::store::{Store, StoreErrorTy};
// use crate::core::store::Error;
use crate::core::uuid::Uuid;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::marker::Unpin;
use std::sync::{Arc,Mutex};

#[derive(Debug, Deserialize, Serialize)]
pub struct Body {
    priority: u32,
    msg: String,
}

pub trait Chunky: Stream<Item=Result<Bytes, &'static str>> + Unpin { }

pub async fn handle<T: Chunky>(
    store: &Mutex<Store>,
    file_storage: &Option<Mutex<FileStorage>>,
    stats: &Mutex<Stats>,
    database: &Mutex<Database>,
    payload: &mut T) -> Result<Arc<Uuid>, &'static str> {

        let mut metadata_string = String::new();
        let mut msg_chunk = BytesMut::new();
        let mut metadata: BTreeMap<String, String> = BTreeMap::new();
        let mut save_to_file = false;
    
        while let Some(chunk) = payload.next().await {
            let chunk = match chunk {
                Ok(chunk) => Ok(chunk),
                Err(error) => {
                    log_err(error, file!(), line!(), "");
                    Err(error)
                }
            }?;
            let mut chunk_string = match String::from_utf8(chunk.to_vec()) {
                Ok(chunk_string) => Ok(chunk_string),
                Err(error) => {
                    log_err(error_codes::COULD_NOT_PARSE_CHUNK, file!(), line!(), error.to_string());
                    Err(error_codes::COULD_NOT_PARSE_CHUNK)
                }
            }?;
            chunk_string = chunk_string.trim_start().to_string();
            // debug!("recieved chunk: {}", chunk_string);
            metadata_string.push_str(&chunk_string);
            if metadata_string.contains("?") {
                // debug!("msg start found!");
                if metadata_string.len() == 0 {
                    return Err(error_codes::MISSING_HEADERS);
                }
                match metadata_string.split_once("?") {
                    Some((metadata_section, msg_section)) => {
                        for pair in metadata_section.to_string().split("&").into_iter() {
                            let kv = pair.trim_end().trim_start().split("=").map(|txt| txt.to_string()).collect::<Vec<String>>();
                            let k = match kv.get(0) {
                                Some(k) => Ok(k.clone()),
                                None => Err(error_codes::MALFORMED_HEADERS)
                            }?;
                            let v = match kv.get(1) {
                                Some(v) => Ok(v.clone()),
                                None => Err(error_codes::MALFORMED_HEADERS)
                            }?;
                            metadata.insert(k, v);
                        };
                        msg_chunk.extend_from_slice(msg_section.as_bytes());
                    },
                    None => {
                        log_err(error_codes::COULD_NOT_PARSE_CHUNK, file!(), line!(), "");
                        return Err(error_codes::COULD_NOT_PARSE_CHUNK)
                    }
                }
                if let Some(save_to_file_value) = metadata.remove("saveToFile") {
                    if save_to_file_value.to_lowercase() == "true" {
                        if let None = file_storage {
                            while let Some(_chunk) = payload.next().await {
    
                            }
                            return Err(error_codes::FILE_STORAGE_NOT_CONFIGURED);
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
                Err(_error) => Err(error_codes::INVALID_PRIORITY)
            },
            None => Err(error_codes::MISSING_PRIORITY)
        }?;

        let (msg_byte_size, msg) = {
            if save_to_file == true {
                if let Some(byte_size_override_str) = metadata.get("byteSizeOverride") {
                    let msg_byte_size = match byte_size_override_str.parse::<u32>() {
                        Ok(byte_size_override) => Ok(byte_size_override),
                        Err(_error) => Err(error_codes::INVALID_BYTESIZE_OVERRIDE)
                    }?;
                    let msg_parse = metadata
                        .iter()
                        .map(|(k, v)| format!("{}={}", k, v))
                        .collect::<Vec<String>>()
                        .join("&");
                    Ok((msg_byte_size, msg_parse))
                } else {
                    Err(error_codes::MISSING_BYTESIZE_OVERRIDE)
                }
            } else {
                while let Some(chunk) = payload.next().await {
                    let chunk = match chunk {
                        Ok(chunk) => Ok(chunk),
                        Err(error) => {
                            log_err(error, file!(), line!(), "");
                            Err(error)
                        }
                    }?;
                    msg_chunk.extend_from_slice(&chunk);
                }
                match String::from_utf8(msg_chunk.to_vec()) {
                    Ok(msg) => Ok((msg.len() as u32, msg)),
                    Err(error) => {
                        log_err(error_codes::COULD_NOT_PARSE_CHUNK, file!(), line!(), error.to_string());
                        return Err(error_codes::COULD_NOT_PARSE_CHUNK)
                    }
                }
            }
        }?;
        let add_result = {
            let mut store = lock(&store)?;
            match store.add(priority, msg_byte_size) {
                Ok(add_result) => Ok(add_result),
                Err(error) => match error.err_ty {
                    StoreErrorTy::ExceedesStoreMax => Err(error_codes::MSG_EXCEEDES_STORE_MAX),
                    StoreErrorTy::ExceedesGroupMax => Err(error_codes::MSG_EXCEEDES_GROUP_MAX),
                    StoreErrorTy::LacksPriority => Err(error_codes::MSG_LACKS_PRIORITY),
                    error => {
                        log_err(error_codes::STORE_ERROR, file!(), line!(), error.to_string());
                        Err(error_codes::STORE_ERROR)
                    }
                },
            }
        }?;
        
        // remove msgs from db
        let mut deleted_count = 0;
        for uuid in add_result.msgs_removed.into_iter() {
            {
                let mut database = lock(&database)?;
                if let Err(error) = database.del(uuid.clone()) {
                    log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                    return Err(error_codes::DATABASE_ERROR)
                }
            }
            if let Some(file_storage) = &file_storage {
                let mut file_storage = lock(&file_storage)?;
                if let Err(error) = rm_from_file_storage(&mut file_storage, &uuid) {
                    // let tmp_err = error.to_string();
                    // log_err(&tmp_err, file!(), line!(), "Could not removed file from file list.");
                    return Err("FS ERROR");
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
                    return Err("FS ERROR")
                }
            } else {
                log_err(error_codes::COULD_NOT_FIND_FILE_STORAGE, file!(), line!(), "");
                return Err(error_codes::COULD_NOT_FIND_FILE_STORAGE);
            }
        }
        {        
            let mut database = lock(&database)?;
            if let Err(error) = database.add(add_result.uuid.clone(), Bytes::copy_from_slice(msg.as_bytes()), msg_byte_size) {
                log_err(error_codes::DATABASE_ERROR, file!(), line!(), error.to_string());
                return Err(error_codes::DATABASE_ERROR);
            }
        }
        Ok(add_result.uuid)
}
