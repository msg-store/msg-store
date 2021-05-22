use crate::db_bridge::DbError;
use std::io;

#[derive(Debug)]
pub enum StoreError {
    Io(io::Error),
    Db(DbError),
    CouldNotConvertDbValue,
    ExceedsStoreLimit,
    ExceedsCollectionLimit,
    LacksPriority,
    OutOfSync,
}
