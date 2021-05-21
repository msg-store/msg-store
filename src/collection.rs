use crate::config::CollectionConfig;
use crate::store::{ByteSize, Priority};
use crate::uuid::Uuid;
use std::collections::BTreeMap;
use std::rc::Rc;

pub type Collections = BTreeMap<Priority, Collection>;
pub type Messages = BTreeMap<Rc<Uuid>, ByteSize>;

pub struct Collection {
  pub priority: Priority,
  pub byte_size: ByteSize,
  pub limit: Option<ByteSize>,
  pub messages: Messages
}
impl Collection {
  pub fn new(config: &CollectionConfig) -> Collection {
    Collection {
      priority: config.priority,
      byte_size: 0,
      limit: config.limit,
      messages: BTreeMap::new()
    }
  }
}