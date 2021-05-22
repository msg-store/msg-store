use crate::msg_data::MsgData;
use bincode::{deserialize, serialize};
use db_key::Key;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::rc::Rc;
use std::time;

#[derive(Debug, Eq, Clone, Copy, Deserialize, Serialize)]
pub struct Uuid {
    priority: u64,
    time_stamp: u128,
    sequence: u64,
}

impl Uuid {
    pub fn from_msg_data(msg_data: &MsgData) -> Rc<Uuid> {
        msg_data.get_uuid().clone()
    }
    pub fn from_string(string: &str) -> Uuid {
        let file_name_str_vec: Vec<&str> = string.split("-").collect();
        Uuid {
            priority: file_name_str_vec[0]
                .parse::<u64>()
                .expect("Could not parse priority"),
            time_stamp: file_name_str_vec[1]
                .parse::<u128>()
                .expect("Could not parse time_stamp"),
            sequence: file_name_str_vec[2]
                .parse::<u64>()
                .expect("Could not parse sequence"),
        }
    }
    pub fn from_vec(array: &Vec<&str>) -> Uuid {
        Uuid {
            priority: array[0].parse::<u64>().expect("Could not parse priority"),
            time_stamp: array[1]
                .parse::<u128>()
                .expect("Could not parse time_stamp"),
            sequence: array[2].parse::<u64>().expect("Could not parse sequence"),
        }
    }
    pub fn to_string(self) -> String {
        format!("{}-{}-{}", self.priority, self.time_stamp, self.sequence)
    }
    pub fn get_priority(self) -> u64 {
        self.priority
    }
    pub fn get_time_stamp(self) -> u128 {
        self.time_stamp
    }
    pub fn get_sequence(self) -> u64 {
        self.sequence
    }
}

impl<'a> From<&'a [u8]> for Uuid {
    fn from(key: &'a [u8]) -> Uuid {
        use std::intrinsics::transmute;
        let key: &Uuid = unsafe { transmute(key.as_ptr()) };
        *key
    }
}

impl AsRef<[u8]> for Uuid {
    fn as_ref(&self) -> &[u8] {
        use std::intrinsics::transmute;
        use std::mem::size_of;
        use std::slice::from_raw_parts;
        unsafe { from_raw_parts(transmute(self), size_of::<Uuid>()) }
    }
}

impl Key for Uuid {
    fn from_u8(key: &[u8]) -> Uuid {
        deserialize(key).unwrap()
    }

    fn as_slice<T, F: Fn(&[u8]) -> T>(&self, f: F) -> T {
        f(&serialize(&self).unwrap())
    }
}

impl Ord for Uuid {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.priority > other.priority {
            Ordering::Greater
        } else if self.priority < other.priority {
            Ordering::Less
        } else {
            if self.time_stamp < other.time_stamp {
                Ordering::Greater
            } else if self.time_stamp > other.time_stamp {
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

impl PartialOrd for Uuid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.priority > other.priority {
            Some(Ordering::Greater)
        } else if self.priority < other.priority {
            Some(Ordering::Less)
        } else {
            if self.time_stamp < other.time_stamp {
                Some(Ordering::Greater)
            } else if self.time_stamp > other.time_stamp {
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

impl PartialEq for Uuid {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
            && self.time_stamp == other.time_stamp
            && self.sequence == other.sequence
    }
}

pub struct IdManager {
    sequence: u64,
    time_stamp: u128,
}
impl IdManager {
    pub fn new() -> IdManager {
        IdManager {
            sequence: 0,
            time_stamp: time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        }
    }
    pub fn new_id(&mut self, priority: &u64) -> Uuid {
        let time_stamp = time::SystemTime::now()
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        if self.time_stamp != time_stamp {
            self.time_stamp = time_stamp;
            self.sequence = 0;
        }
        let sequence = {
            let sequence = self.sequence;
            self.sequence += 1;
            sequence
        };
        Uuid {
            priority: *priority,
            sequence,
            time_stamp,
        }
    }
}
