use std::cmp::Ordering;
use std::sync::Arc;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Uuid {
    pub priority: u32,
    pub timestamp: u128,
    pub sequence: u32,
    pub node_id: u16
}
impl Uuid {
    pub fn to_string(&self) -> String {
        format!("{}-{}-{}-{}", self.priority, self.timestamp, self.sequence, self.node_id)
    }
    pub fn from_string(id: &str) -> Result<Arc<Uuid>, String> {
        let split_str = id.split("-").collect::<Vec<&str>>();
        let priority: u32 = match split_str[0].parse() {
            Ok(priority) => priority,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        let timestamp: u128 = match split_str[1].parse() {
            Ok(timestamp) => timestamp,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        let sequence: u32 = match split_str[2].parse() {
            Ok(sequence) => sequence,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        let node_id: u16 = match split_str[3].parse() {
            Ok(sequence) => sequence,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        Ok(Arc::new(Uuid {
            priority,
            timestamp,
            sequence,
            node_id
        }))
    }
}

impl PartialOrd for Uuid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.priority > other.priority {
            Some(Ordering::Greater)
        } else if self.priority < other.priority {
            Some(Ordering::Less)
        } else {
            if self.timestamp < other.timestamp {
                Some(Ordering::Greater)
            } else if self.timestamp > other.timestamp {
                Some(Ordering::Less)
            } else {
                if self.sequence < other.sequence {
                    Some(Ordering::Greater)
                } else if self.sequence > other.sequence {
                    Some(Ordering::Less)
                } else {
                    Some(Ordering::Equal)
                }
            }
        }
    }
}

impl Ord for Uuid {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority > other.priority {
            Ordering::Greater
        } else if self.priority < other.priority {
            Ordering::Less
        } else {
            if self.timestamp < other.timestamp {
                Ordering::Greater
            } else if self.timestamp > other.timestamp {
                Ordering::Less
            } else {
                if self.sequence < other.sequence {
                    Ordering::Greater
                } else if self.sequence > other.sequence {
                    Ordering::Less
                } else {
                    Ordering::Equal
                }
            }
        }
    }
}

pub struct UuidManager {
    pub timestamp: u128,
    pub sequence: u32,
    pub node_id: u16
}
impl UuidManager {
    pub fn default() -> UuidManager {
        UuidManager {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).expect("failed to get duration").as_nanos(),
            sequence: 1,
            node_id: 0
        }
    }
    pub fn new(node_id: Option<u16>) -> UuidManager {
        let node_id = match node_id {
            Some(node_id) => node_id,
            None => 0
        };
        let mut manager = UuidManager::default();
        manager.node_id = node_id;
        manager
    }
    pub fn next(&mut self, priority: u32) -> Arc<Uuid> {
        let nano = SystemTime::now().duration_since(UNIX_EPOCH).expect("failed to get duration").as_nanos();
        if nano != self.timestamp {
            self.timestamp = nano;
            self.sequence = 1;
        } else {
            self.sequence += 1;
        }
        Arc::new(Uuid {
            priority,
            timestamp: self.timestamp,
            sequence: self.sequence,
            node_id: self.node_id           
        })
    }
}