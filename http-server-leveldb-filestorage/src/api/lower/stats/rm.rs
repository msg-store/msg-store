use crate::api::lower::stats::Stats;
use crate::api::lower::lock;
use std::sync::Mutex;

pub fn handle(stats_mutex: &Mutex<Stats>) -> Result<Stats, &'static str> {
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
