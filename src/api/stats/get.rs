use crate::api::lock;
use crate::api::stats::Stats;
use std::sync::Mutex;

pub fn handle(stats_mutex: &Mutex<Stats>) -> Result<Stats, &'static str> {
    let stats = {
        let stats = lock(stats_mutex)?;
        stats.clone()
    };
    Ok(stats)
}
