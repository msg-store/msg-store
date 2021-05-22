use std::path::PathBuf;

pub struct StoreConfig {
    pub dir: PathBuf,
    pub limit: Option<u64>,
    pub collections: Option<Vec<CollectionConfig>>,
}

pub struct CollectionConfig {
    pub priority: u64,
    pub limit: Option<u64>,
}
impl CollectionConfig {
    pub fn new(priority: u64, limit: Option<u64>) -> CollectionConfig {
        CollectionConfig { priority, limit }
    }
}
