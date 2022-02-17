use crate::api::stats::Stats;
use crate::api::{ApiError, NoErr, lock};
use std::sync::Mutex;

pub fn try_rm(stats_mutex: &Mutex<Stats>) -> Result<Stats, ApiError<NoErr, NoErr>> {
    let stats = {
        let mut stats = lock(stats_mutex)?;
        let old_stats = stats.clone();
        stats.inserted = 0;
        stats.deleted = 0;
        stats.pruned = 0;
        old_stats
    };
    Ok(stats)
}
