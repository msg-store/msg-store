use actix_web::web::{ Data, Payload };
use actix_web::{HttpResponse};
use bytes::Bytes;
use crate::AppData;
use msg_store_server_api::msg::add::{handle, Chunky, AddErrorTy, MsgError};
use futures::{Stream, StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::task::Poll;

pub struct PayloadBridge(Payload);

impl Stream for PayloadBridge {
    type Item = Result<Bytes, &'static str>;
    fn poll_next(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let chunk_option = match self.0.poll_next_unpin(cx) {
            Poll::Ready(chunk_option) => chunk_option,
            Poll::Pending => return Poll::Pending
        };
        let chunk_result = match chunk_option {
            Some(chunk_result) => chunk_result,
            None => return Poll::Ready(None)
        };
        match chunk_result {
            Ok(chunk) => Poll::Ready(Some(Ok(Bytes::copy_from_slice(&chunk)))),
            Err(_error) => {
                Poll::Ready(Some(Err("PayloadError")))
            }
        }
    }
}

impl Chunky for PayloadBridge { }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ReturnBody {
    uuid: String,
}

const ROUTE: &'static str = "POST /api/msg";
pub async fn http_handle(data: Data<AppData>, body: Payload) -> HttpResponse {
    info!("{}", ROUTE);
    match handle(&data.store, &data.file_storage, &data.stats, &data.db, PayloadBridge(body)).await {
        Ok(uuid) => HttpResponse::Ok().json(ReturnBody { uuid: uuid.to_string() }),
        Err(error) => {
            match error.err_ty {
                AddErrorTy::MsgError(msg_error) => {
                    match msg_error {
                        MsgError::InvalidBytesizeOverride |
                        MsgError::InvalidPriority |
                        MsgError::MalformedHeaders |
                        MsgError::MissingBytesizeOverride |
                        MsgError::MissingHeaders |
                        MsgError::MissingPriority => {
                            info!("{} 400 {}", ROUTE, msg_error);
                            HttpResponse::BadRequest().body(msg_error.to_string())
                        },
                        MsgError::CouldNotGetNextChunkFromPayload |
                        MsgError::CouldNotParseChunk => {
                            info!("{} 400 {}", ROUTE, msg_error);
                            HttpResponse::BadRequest().body(msg_error.to_string())
                        },
                        MsgError::FileStorageNotConfigured => {
                            info!("{} 403 {}", ROUTE, msg_error);
                            HttpResponse::Forbidden().body(msg_error.to_string())
                        },
                        MsgError::MsgExceedesGroupMax |
                        MsgError::MsgExceedesStoreMax |
                        MsgError::MsgLacksPriority => {
                            info!("{} 409 {}", ROUTE, msg_error);
                            HttpResponse::BadRequest().body(msg_error.to_string())
                        }
                    }
                },
                AddErrorTy::CouldNotFindFileStorage |
                _ => {
                    error!("ROUTE {}", error);
                    exit(1)
                }
            }
        }
    }
}
