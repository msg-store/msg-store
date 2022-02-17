use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    ExceedesStoreMax,
    ExceedesGroupMax,
    LacksPriority,
    SyncError,
    UuidError
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}