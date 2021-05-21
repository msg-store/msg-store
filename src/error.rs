use std::io;
use crate::db_bridge::DbError;

#[derive(Debug)]
pub enum StoreError {
  Io(io::Error),
  Db(DbError),
  CouldNotConvertDbValue,
  ExceedsStoreLimit,
  ExceedsCollectionLimit,
  LacksPriority,
  OutOfSync
}
