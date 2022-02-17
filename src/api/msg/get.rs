use crate::api::{lock, Database, Either, ApiError, ApiErrorTy, NoErr};
use crate::api::file_storage::{get_buffer, FileStorage, ReturnBody };
use crate::core::store::Store;
use crate::core::uuid::Uuid;
use crate::api_err;
use std::error::Error;
use std::sync::{Arc, Mutex};

pub fn try_get<E: Error>(
    store: &Mutex<Store>,
    database_mutex: &Mutex<Database<E>>,
    file_storage_option: &Option<Mutex<FileStorage>>,
    uuid_option: Option<Arc<Uuid>>,
    priority_option: Option<u32>,
    reverse_option: bool
) -> Result<Option<Either<ReturnBody, String>>, ApiError<E, NoErr>> {
    let uuid = {
        let store = lock(&store)?;
        match store.get(uuid_option, priority_option, reverse_option) {
            Ok(uuid) => match uuid {
                Some(uuid) => Ok(uuid),
                None => return Ok(None)
            },
            Err(error) => Err(api_err!(ApiErrorTy::StoreError(error)))
        }
    }?;
    let msg = {
        let mut database = lock(&database_mutex)?;
        match database.get(uuid.clone()) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(api_err!(ApiErrorTy::DbError(error)))
        }
    }?;
    if let Some(file_storage_mutex) = &file_storage_option {
        let file_storage = lock(file_storage_mutex)?;
        if file_storage.index.contains(&uuid) {
            let (file_buffer, file_size) = match get_buffer(&file_storage.path, &uuid) {
                Ok(buffer_option) => Ok(buffer_option),
                Err(error) => {
                    Err(api_err!(ApiErrorTy::FileStorageError(error)))
                }
            }?;
            let msg_header = match String::from_utf8(msg.to_vec()) {
                Ok(msg_header) => Ok(msg_header),
                Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
            }?;
            let body = ReturnBody::new(format!("uuid={}&{}?", uuid.to_string(), msg_header), file_size, file_buffer);
            Ok(Some(Either::A(body)))
        } else {
            let msg = match String::from_utf8(msg.to_vec()) {
                Ok(msg) => Ok(msg),
                Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
            }?;
            Ok(Some(Either::B(format!("uuid={}?{}", uuid.to_string(), msg))))
        }
    } else {
        let msg = match String::from_utf8(msg.to_vec()) {
            Ok(msg) => Ok(msg),
            Err(error) => Err(api_err!(ApiErrorTy::CouldNotParseChunk, error))
        }?;
        Ok(Some(Either::B(format!("uuid={}?{}", uuid.to_string(), msg))))
    }
}
