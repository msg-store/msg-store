use crate::{db_bridge::Bridge, msg_data::MsgData};
use crate::collection::{Collections, Collection};
use crate::config::{CollectionConfig,StoreConfig};
use crate::error::StoreError;
use crate::uuid::{IdManager, Uuid};
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::rc::Rc;

pub type ByteSize = u64;
pub type Priority = u64;

/* 

  ## ADD ##
  first get the byte size of the msg
  check if the new messages pushes the store over the limit
    get a list of messages to be deleted from all collections
  check if the new message pushes the collection over the limit
    get a list of messages to be deleted from the collection

*/

#[derive(Debug, PartialEq, Eq)]
pub struct Packet {
  pub id: String,
  pub msg: Vec<u8>
}

pub struct InsertPacket {
  priority: Priority,
  msg: Vec<u8>
}

struct InnerStoreConfig {
  pub dir: PathBuf,
  pub limit: Option<u64>,
  pub collections: Option<HashMap<Priority, CollectionConfig>>
}

pub struct Store<B: Bridge> {
  bridge: B,
  byte_size: u64,
  config: InnerStoreConfig,
  id_manager: IdManager,
  collections: Collections
}

impl<B: Bridge> Store<B> {

  pub fn new(config: StoreConfig, bridge: B) -> Result<Store<B>, StoreError> {
    let packets: Vec<MsgData> = match bridge.read() {
        Ok(packets) => Ok(packets),
        Err(db_error) => Err(StoreError::Db(db_error))
    }?;
    let mut collections: Collections = BTreeMap::new();
    let mut byte_size: ByteSize = 0;
    let inner_config: InnerStoreConfig = InnerStoreConfig {
      dir: config.dir,
      limit: config.limit,
      collections: match config.collections {
        Some(collections_vec) => {
          let mut map: HashMap<Priority, CollectionConfig> = HashMap::new();
          for config in collections_vec {
            map.insert(config.priority, config);
          }
          Some(map)
        },
        None => None
      }
    };
    for packet in packets {
      let uuid = packet.get_uuid();
      let msg_byte_size = packet.get_byte_size();
      let priority = uuid.get_priority();
      if let Some(col_ref_mut) = collections.get_mut(&priority) {
        col_ref_mut.byte_size += msg_byte_size;
        col_ref_mut.messages.insert(uuid.clone(), msg_byte_size);
      } else {
        /* TODO: check for config */
        let mut col: Collection = Collection::new(&CollectionConfig::new(priority, None));
        col.byte_size += msg_byte_size;
        col.messages.insert(uuid.clone(), msg_byte_size);
        collections.insert(priority, col);
      }
      byte_size += msg_byte_size;
    }
    Ok(Store {
      bridge,
      config: inner_config,
      id_manager: IdManager::new(),
      byte_size,
      // collection_list: RefCell::new(collection_list),
      collections
    })
  }

  pub fn add(&mut self, priority: &u64, msg: &Vec<u8>) -> Result<(), StoreError> {    
    let msg_byte_size = msg.len() as u64;
    if let Some(limit) = self.config.limit {
      if msg_byte_size > limit {
        return Err(StoreError::ExceedsStoreLimit);
      }
    }
    let uuid = Rc::new(self.id_manager.new_id(&priority));
    if let Some(col_ref_mut) = self.collections.get_mut(&priority) {
      if let Some(limit) = col_ref_mut.limit {
        if msg_byte_size > limit {
          return Err(StoreError::ExceedsCollectionLimit);
        }
      }
    }
    // get available byte size
    let mut used_byte_size: u64 = 0;
    for (col_priority, col_ref) in self.collections.iter() {
      used_byte_size += col_ref.byte_size;
      if col_priority < priority {
        break;
      }
    }
    if let Some(limit) = self.config.limit {
      let available_byte_size = limit - used_byte_size;
      if msg_byte_size > available_byte_size {
        return Err(StoreError::LacksPriority);
      }
    }
    // validate that the new collection size does not exceed collection limit
    if let Some(col_ref_mut) = self.collections.get_mut(priority) {
      let proposed_byte_size = col_ref_mut.byte_size + msg_byte_size;
      if let Some(limit) = col_ref_mut.limit {
        if proposed_byte_size > limit {
          let excess_bytes = proposed_byte_size - limit;
          let mut bytes_removed: u64 = 0;
          let mut ids_removed: Vec<Rc<Uuid>> = vec![];
          for (uuid, byte_size) in col_ref_mut.messages.iter() {
            bytes_removed += byte_size;
            ids_removed.push(uuid.clone());
            if bytes_removed >= excess_bytes {
              break;
            }
          }
          // remove id's from collection
          for uuid in ids_removed {
            col_ref_mut.messages.remove(&(*uuid));
          }
          // decrease bytes from collection data
          col_ref_mut.byte_size -= bytes_removed;
          // decrease bytes from store
          self.byte_size -= bytes_removed;
        }
      }      
    }
    // validate that the store does not exceed the store limit
    if let Some(limit) = self.config.limit {
      let proposed_byte_size = self.byte_size + msg_byte_size;
      if proposed_byte_size > limit {
        let excess_bytes = proposed_byte_size - limit;
        let mut bytes_removed: u64 = 0;
        let mut whole_collections_to_removed: Vec<Priority> = vec![];
        for (priority, col_ref_mut) in self.collections.iter_mut().rev() {
          let bytes_still_needed_to_be_removed = excess_bytes - bytes_removed;
          if bytes_still_needed_to_be_removed >= col_ref_mut.byte_size {
            whole_collections_to_removed.push(*priority);
            bytes_removed += col_ref_mut.byte_size;
            continue;
          }
          let mut bytes_removed_from_col: u64 = 0;
          let mut uuids_removed: Vec<Rc<Uuid>> = vec![];
          for (uuid, byte_size) in col_ref_mut.messages.iter() {
            bytes_removed += byte_size;
            bytes_removed_from_col += byte_size;
            uuids_removed.push(uuid.clone());
            if bytes_removed >= excess_bytes {
              col_ref_mut.byte_size -= bytes_removed_from_col;
              break;
            }
          }
          for uuid in uuids_removed {
            col_ref_mut.messages.remove(&uuid);
          }
          if bytes_removed >= excess_bytes {
            break;
          }
        }
        for priority in whole_collections_to_removed {
          self.collections.remove(&priority);
        }
        self.byte_size -= bytes_removed;
      }
    }
    // add msg data to collection
    if let Some(col_ref_mut) = self.collections.get_mut(priority) {
      col_ref_mut.byte_size += msg_byte_size;
      col_ref_mut.messages.insert(uuid.clone(), msg_byte_size);
    } else {
      let mut col;
      if let Some(collection_configs) = &self.config.collections {
        if let Some(config) = collection_configs.get(&priority) {
          col = Collection::new(&config);
        } else {
          col = Collection::new(&CollectionConfig::new(*priority, None)); 
        }
      } else {
        col = Collection::new(&CollectionConfig::new(*priority, None));
      }
      col.byte_size += msg_byte_size;
      col.messages.insert(uuid.clone(), msg_byte_size);
      self.collections.insert(*priority, col);
    }
    if let Err(error) = self.bridge.put(uuid.clone(), &msg) {
        if let Some(col_ref_mut) = self.collections.get_mut(priority) {
            col_ref_mut.byte_size -= msg_byte_size;
            col_ref_mut.messages.remove(&uuid);
        } else {
            return Err(StoreError::OutOfSync)
        }
        return Err(StoreError::Db(error))
    }
    self.byte_size += msg_byte_size;
    Ok(())
  }

  pub fn del(&mut self, id: &str) -> Result<(), StoreError> {
    let uuid = Rc::new(Uuid::from_string(id));
    let priority = uuid.get_priority();
    let mut remove_collection = false;
    if let Some(col_ref_mut) = self.collections.get_mut(&priority) {
      let bytes_removed = {
        col_ref_mut.messages.remove(&uuid)
      };
      if let Some(bytes_removed) = bytes_removed {
        col_ref_mut.byte_size -= bytes_removed;
        self.byte_size -= bytes_removed;
        if let Err(error) = self.bridge.del(uuid) {
            col_ref_mut.byte_size += bytes_removed;
            self.byte_size += bytes_removed;
            return Err(StoreError::Db(error))
        }
        if col_ref_mut.messages.len() == 0 {
          remove_collection = true;
        }
      }
    }
    if remove_collection {
      self.collections.remove(&priority);
    }
    Ok(())
  }

  pub fn get(&self) -> Result<Option<Packet>, StoreError> {
    if let Some((_prioriy,  col_ref)) = self.collections.iter().next() {
      if let Some((uuid, _byte_size)) = col_ref.messages.iter().next() {
        let id = uuid.to_string();
        return match self.bridge.get(uuid.clone()) {
            Ok(msg_option) => match msg_option {
                Some(msg) => Ok(Some(Packet { id, msg })),
                None => Ok(None)
            },
            Err(error) => Err(StoreError::Db(error))
        }
      } else {
        return Err(StoreError::OutOfSync)
      }
    }
    Ok(None)
  }

}

