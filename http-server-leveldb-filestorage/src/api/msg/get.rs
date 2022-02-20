use actix_web::{ HttpResponse, Error };
use actix_web::web::{ Data, Query, Bytes };
use crate::AppData;
use futures::stream::{Stream, StreamExt};
use futures::task::{Context, Poll};
use log::{error, info};
use server_api::msg::get::{handle, ReturnBody as ApiReturn};
use server_api::Either;
use msg_store_uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::process::exit;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MsgData {
    uuid: String,
    msg: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Info {
    uuid: Option<String>,
    priority: Option<u32>,
    reverse: Option<bool>,
}

pub struct ReturnBody {
    stream: ApiReturn
}
impl ReturnBody {
    pub fn new(stream: ApiReturn) -> ReturnBody {
        ReturnBody {
            stream
        }
    }
}
impl Stream for ReturnBody {
    type Item = Result<Bytes, Error>;
    fn poll_next(
        mut self: Pin<&mut Self>, 
        cx: &mut Context<'_>
    ) -> Poll<Option<Self::Item>> {
        let chunk_option = match self.stream.poll_next_unpin(cx) {
            Poll::Ready(chunk_option) => chunk_option,
            Poll::Pending => { return Poll::Pending }
        };
        let chunk_result = match chunk_option {
            Some(chunk_result) => chunk_result,
            None => return Poll::Ready(None)
        };
        match chunk_result {
            Ok(bytes) => Poll::Ready(Some(Ok(Bytes::copy_from_slice(&bytes)))),
            Err(_error_code) => {
                Poll::Ready(Some(Err(Error::from(()))))
            }
        }
    }
}

const ROUTE: &'static str = "GET /api/msg";
pub fn http_handle(data: Data<AppData>, info: Query<Info>) -> HttpResponse {
    info!("{}", ROUTE);
    let uuid = if let Some(uuid_string) = &info.uuid {
        match Uuid::from_string(&uuid_string) {
            Ok(uuid) => Some(uuid),
            Err(err) => {
                info!("{} 400 {}", ROUTE, err);
                return HttpResponse::BadRequest().body(err.err_ty.to_string());
            } 
        }
    } else {
        None
    };
    let priority = info.priority;
    let reverse = if let Some(reverse) = info.reverse {
        reverse
    } else {
        false
    };
    let msg_option = match handle(
        &data.store, 
        &data.db, 
        &data.file_storage, 
        uuid, 
        priority, 
        reverse) {
        Ok(message_option) => message_option,
        Err(err) => {
            error!("{} {}", ROUTE, err);
            exit(1);
        }
    };
    let msg_type = match msg_option {
        Some(msg_type) => msg_type,
        None => {
            info!("{} 200 No Message", ROUTE);
            return HttpResponse::Ok().finish()
        }
    };
    let buffer = match msg_type {
        Either::A(buffer) => buffer,
        Either::B(msg) => {
            info!("{} 200 {}", ROUTE, msg);
            return HttpResponse::Ok().body(msg)
        }
    };
    info!("{} 200 {}", ROUTE, buffer.header);
    HttpResponse::Ok().streaming(ReturnBody::new(buffer))
}
