
#[derive(Debug)]
pub struct DbError(String);

#[derive(Debug)]
pub enum Error {
    ExceedesStoreMax,
    ExceedesGroupMax,
    LacksPriority,
    DbError(DbError),
    SyncError
}