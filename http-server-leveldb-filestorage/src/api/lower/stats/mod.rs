pub mod rm;
pub mod get;
pub mod set;

use serde::Serialize;
use serde_json::json;
use std::fmt::Display;

#[derive(Debug, Serialize, Clone, Copy)]
pub struct Stats {
    pub inserted: u32,
    pub deleted: u32,
    pub pruned: u32
}
impl Stats {
    pub fn new() -> Stats {
        Stats {
            inserted: 0,
            deleted: 0,
            pruned: 0
        }
    }
    pub fn add(&mut self, inserted: u32, deleted: u32, pruned: u32) {
        self.inserted += inserted;
        self.deleted += deleted;
        self.pruned += pruned;
    }
    pub fn replace(&mut self, inserted: u32, deleted: u32, pruned: u32) {
        self.inserted = inserted;
        self.deleted = deleted;
        self.pruned = pruned;
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json_value = json!(self);
        write!(f, "{}", json_value)
    }
}