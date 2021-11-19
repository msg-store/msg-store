use std::fmt::Display;

#[derive(Debug)]
pub struct DbError(pub String);
impl Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum Error {
    ExceedesStoreMax,
    ExceedesGroupMax,
    LacksPriority,
    DbError(DbError),
    SyncError
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DbError(db_error) => write!(f, "{}", db_error),
            _ => write!(f, "{}", self)
        }
        
    }
}