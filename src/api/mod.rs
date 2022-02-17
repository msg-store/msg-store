use crate::api::configuration::ConfigError;
use crate::api::file_storage::FsError;
use crate::core::store::StoreError;
use crate::database::Db;
use std::error::Error;
use std::fmt::Display;
use std::sync::{Mutex, MutexGuard};

// pub mod export;

pub mod configuration;
pub mod file_storage;
pub mod group;
pub mod group_defaults;
pub mod msg;
pub mod stats;
pub mod store;

#[derive(Debug, PartialEq, Eq)]
pub struct NoErr;
impl Display for NoErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
impl Error for NoErr {

}

#[derive(Debug, PartialEq, Eq)]
pub enum MsgError {
    MsgExceedesStoreMax,
    MsgExceedsGroupMax,
    MsgLacksPriority,
    MissingHeaders,
    InvalidHeaders,
    InvalidPriority,
    MissingPriority,
    InvalidByteSizeOverride,
    MissingByteSizeOverride,
    MalformedHeaders,
    FileStorageNotConfigured
}
impl Display for MsgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ApiErrorTy<E: Error, P: Error> {
    CouldNotLockMutex,
    StoreError(StoreError),
    DbError(E),
    FileStorageError(FsError),
    ConfigurationError(ConfigError),
    PayloadError(P),
    FileStorageNotFound,
    CouldNotParseChunk,
    MsgError(MsgError)
}

#[derive(Debug, PartialEq, Eq)]
pub struct ApiError<E: Error, P: Error> {
    error_ty: ApiErrorTy<E, P>,
    file: &'static str,
    line: u32,
    msg: Option<String>
}

impl<E: Error, P: Error> Display for ApiError<E, P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}", self)
    }
}

impl<E: Error, P: Error> Error for ApiError<E, P> { }

#[macro_export]
macro_rules! api_err {
    ($err_ty:expr) => {
        ApiError {
            error_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $err:expr) => {
        ApiError {
            error_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($err.to_string())
        }
    };
}


pub type Database<E> = Box<dyn Db<E>>;
pub enum Either<A, B> {
    A(A),
    B(B)
}

pub fn lock<'a, T: Send + Sync, E: Error, P: Error>(item: &'a Mutex<T>) -> Result<MutexGuard<'a, T>, ApiError<E, P>> {
    match item.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(_) => Err(api_err!(ApiErrorTy::CouldNotLockMutex))
    }
}
