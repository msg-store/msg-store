
#[derive(Debug)]
pub struct DbError(pub String);

#[derive(Debug)]
pub enum Error {
    ExceedesStoreMax,
    ExceedesGroupMax,
    LacksPriority,
    DbError(DbError),
    SyncError
}