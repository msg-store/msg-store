use crate::api::{ApiError, NoErr, lock};
use crate::api::stats::Stats;
use std::sync::Mutex;

pub fn try_get(stats_mutex: &Mutex<Stats>) -> Result<Stats, ApiError<NoErr, NoErr>> {
    let stats = {
        let stats = lock(stats_mutex)?;
        stats.clone()
    };
    Ok(stats)
}
