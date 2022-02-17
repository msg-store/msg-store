use crate::api::lock;
use crate::api::stats::Stats;
use std::sync::Mutex;

pub fn add_to_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u64>, del_o: Option<u64>, prnd_o: Option<u64>) -> Result<Stats, &'static str> {
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

pub fn replace_stats(stats_mutex: &Mutex<Stats>, insrt_o: Option<u64>, del_o: Option<u64>, prnd_o: Option<u64>) -> Result<Stats, &'static str> {
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

pub fn handle(
    stats_mutex: &Mutex<Stats>,
    add: bool,
    inserted: Option<u64>,
    deleted: Option<u64>,
    pruned: Option<u64>
) -> Result<Stats, &'static str> {
    if add {
        add_to_stats(stats_mutex, inserted, deleted, pruned)
    } else {
        replace_stats(stats_mutex, inserted, deleted, pruned)
    }
}
