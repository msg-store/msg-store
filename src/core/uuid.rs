use std::cmp::Ordering;
use std::fmt::Display;
use std::sync::Arc;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

#[derive(Debug)]
pub enum UuidErrorTy {
    InvalidPriority,
    InvalidTimestamp,
    InvalidSequence,
    InvalidNodeId,
    InvalidFormat
}
impl Display for UuidErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug)]
pub struct UuidError {
    pub err_ty: UuidErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}
impl Display for UuidError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
       if let Some(msg) = &self.msg {
            write!(f, "UUID_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "UUID_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! uuid_error {
    ($err_ty:expr) => {
        UuidError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        UuidError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

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
    pub fn from_string(id: &str) -> Result<Arc<Uuid>, UuidError> {
        let split_str = id.split("-").collect::<Vec<&str>>();
        if split_str.len() != 4 {
            return Err(uuid_error!(UuidErrorTy::InvalidFormat));
        }
        let priority: u32 = match split_str[0].parse() {
            Ok(priority) => priority,
            Err(error) => {
                return Err(uuid_error!(UuidErrorTy::InvalidPriority, error))
            }
        };
        let timestamp: u128 = match split_str[1].parse() {
            Ok(timestamp) => timestamp,
            Err(error) => {
                return Err(uuid_error!(UuidErrorTy::InvalidTimestamp, error))
            }
        };
        let sequence: u32 = match split_str[2].parse() {
            Ok(sequence) => sequence,
            Err(error) => {
                return Err(uuid_error!(UuidErrorTy::InvalidSequence, error))
            }
        };
        let node_id: u16 = match split_str[3].parse() {
            Ok(sequence) => sequence,
            Err(error) => {
                return Err(uuid_error!(UuidErrorTy::InvalidNodeId, error))
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

#[derive(Debug)]
pub enum UuidManagerErrorTy {
    SystemTimeError
}
impl Display for UuidManagerErrorTy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug)]
pub struct UuidManagerError {
    pub err_ty: UuidManagerErrorTy,
    pub file: &'static str,
    pub line: u32,
    pub msg: Option<String>
}
impl Display for UuidManagerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(msg) = &self.msg {
            write!(f, "UUID_MANAGER_ERROR: {}. file: {}, line: {}, msg: {}", self.err_ty, self.file, self.line, msg)
        } else {
            write!(f, "UUID_MANAGER_ERROR: {}. file: {}, line: {}.", self.err_ty, self.file, self.line)
        }
    }   
}

macro_rules! uuid_manager_error {
    ($err_ty:expr) => {
        UuidManagerError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: None
        }
    };
    ($err_ty:expr, $msg:expr) => {
        UuidManagerError {
            err_ty: $err_ty,
            file: file!(),
            line: line!(),
            msg: Some($msg.to_string())
        }
    };
}

pub struct UuidManager {
    pub timestamp: u128,
    pub sequence: u32,
    pub node_id: u16
}
impl UuidManager {
    pub fn default() -> Result<UuidManager, UuidManagerError> {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_nanos()),
            Err(error) => Err(uuid_manager_error!(UuidManagerErrorTy::SystemTimeError, error))
        }?;
        Ok(UuidManager {
            timestamp,
            sequence: 1,
            node_id: 0
        })
    }
    pub fn new(node_id: Option<u16>) -> Result<UuidManager, UuidManagerError> {
        let node_id = match node_id {
            Some(node_id) => node_id,
            None => 0
        };
        let mut manager = UuidManager::default()?;
        manager.node_id = node_id;
        Ok(manager)
    }
    pub fn next(&mut self, priority: u32) -> Result<Arc<Uuid>, UuidManagerError> {
        let nano = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_nanos()),
            Err(error) => Err(uuid_manager_error!(UuidManagerErrorTy::SystemTimeError, error))
        }?;
        if nano != self.timestamp {
            self.timestamp = nano;
            self.sequence = 1;
        } else {
            self.sequence += 1;
        }
        Ok(Arc::new(Uuid {
            priority,
            timestamp: self.timestamp,
            sequence: self.sequence,
            node_id: self.node_id           
        }))
    }
}
