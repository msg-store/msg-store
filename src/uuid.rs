
#[cfg(feature = "level")]
use serde::{Serialize, Deserialize};
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

#[cfg_attr(feature = "level", derive(Serialize, Deserialize))]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Uuid {
    pub timestamp: u128,
    pub sequence: u32
}
impl Uuid {
    pub fn to_string(&self) -> String {
        format!("{}-{}", self.timestamp, self.sequence)
    }
    pub fn from_string(id: &str) -> Result<Uuid, String> {
        let split_str = id.split("-").collect::<Vec<&str>>();
        
        let timestamp: u128 = match split_str[0].parse() {
            Ok(timestamp) => timestamp,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        let sequence: u32 = match split_str[1].parse() {
            Ok(sequence) => sequence,
            Err(error) => {
                return Err(error.to_string())
            }
        };
        Ok(Uuid { 
            timestamp,
            sequence
        })
    }
}

pub struct UuidManager {
    pub timestamp: u128,
    pub sequence: u32
}
impl UuidManager {
    pub fn default() -> UuidManager {
        UuidManager {
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).expect("failed to get duration").as_nanos(),
            sequence: 1
        }
    }
    pub fn next(&mut self) -> Uuid {
        let nano = SystemTime::now().duration_since(UNIX_EPOCH).expect("failed to get duration").as_nanos();
        if nano != self.timestamp {
            self.timestamp = nano;
            self.sequence = 1;
        } else {
            self.sequence += 1;
        }
        Uuid {
            timestamp: self.timestamp,
            sequence: self.sequence            
        }
    }
}



