use crate::api::{ApiError, NoErr, lock};
use crate::api::stats::Stats;
use std::sync::Mutex;

pub fn add_to_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u32>, del_o: Option<u32>, prnd_o: Option<u32>) -> Result<Stats, ApiError<NoErr, NoErr>> {
    let mut stats = lock(stats_mutex)?;
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

pub fn replace_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u32>, del_o: Option<u32>, prnd_o: Option<u32>) -> Result<Stats, ApiError<NoErr, NoErr>> {
    let mut stats = lock(stats_mutex)?;
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

pub fn try_set(
    stats_mutex: &Mutex<Stats>,
    add: bool,
    inserted: Option<u32>,
    deleted: Option<u32>,
    pruned: Option<u32>
) -> Result<Stats, ApiError<NoErr, NoErr>> {
    if add {
        add_to_stats(stats_mutex, inserted, deleted, pruned)
    } else {
        replace_stats(stats_mutex, inserted, deleted, pruned)
    }
}
