use crate::stats::Stats;
use std::fmt::Display;
use std::sync::Mutex;


#[derive(Debug)]
pub enum ErrTy {
    LockingError
}
impl Display for ErrTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LockingError => write!(f, "{:#?}", self)
        }
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub err_ty: ErrTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "SET_STATS_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "SET_STATS_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! api_error {
    ($err_ty:expr) => {
        ApiError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        ApiError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub fn add_to_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u64>, del_o: Option<u64>, prnd_o: Option<u64>) -> Result<Stats, ApiError> {
    let mut stats = match stats_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let stats_old = stats.clone();
    if let Some(inserted) = insrt_o {
        stats.inserted += inserted;
    }
    if let Some(deleted) = del_o {
        stats.deleted += deleted;
    }
    if let Some(pruned) = prnd_o {
        stats.pruned += pruned;
    }
    Ok(stats_old)
}

pub fn replace_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u64>, del_o: Option<u64>, prnd_o: Option<u64>) -> Result<Stats, ApiError> {
    let mut stats = match stats_mutex.lock() {
        Ok(gaurd) => Ok(gaurd),
        Err(err) => Err(api_error!(ErrTy::LockingError, err))
    }?;
    let stats_old = stats.clone();
    if let Some(inserted) = insrt_o {
        stats.inserted = inserted;
    }
    if let Some(deleted) = del_o {
        stats.deleted = deleted;
    }
    if let Some(pruned) = prnd_o {
        stats.pruned = pruned;
    }
    Ok(stats_old)
}

pub fn handle(
    stats_mutex: &Mutex<Stats>,
    add: bool,
    inserted: Option<u64>,
    deleted: Option<u64>,
    pruned: Option<u64>
) -> Result<Stats, ApiError> {
    if add {
        add_to_stats(stats_mutex, inserted, deleted, pruned)
    } else {
        replace_stats(stats_mutex, inserted, deleted, pruned)
    }
}
