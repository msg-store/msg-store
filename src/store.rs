use crate::collection::{Collection, Collections};
use crate::config::{CollectionConfig, StoreConfig};
use crate::error::StoreError;
use crate::uuid::{IdManager, Uuid};
use crate::{db_bridge::Bridge, msg_data::MsgData};
use std::collections::BTreeMap;
use std::collections::HashMap;
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
    pub msg: Vec<u8>,
}

pub struct InsertPacket {
    priority: Priority,
    msg: Vec<u8>,
}

struct InnerStoreConfig {
    pub dir: PathBuf,
    pub limit: Option<u64>,
    pub collections: Option<HashMap<Priority, CollectionConfig>>,
}

pub enum EventMsg {
    MsgAdded,
    MsgDeleted,
    MsgBurned(Rc<Uuid>, Priority, ByteSize),
    CollectionBurned(Priority, u32, ByteSize),
    MsgDumped(Rc<Uuid>, ByteSize),
    MsgImported(Rc<Uuid>, Priority, ByteSize)
}

pub struct Store<B: Bridge> {
    bridge: B,
    byte_size: u64,
    config: InnerStoreConfig,
    id_manager: IdManager,
    collections: Collections,
    listener: Option<Box<dyn Fn(EventMsg)>>
}

impl<B: Bridge> Store<B> {
    pub fn new(config: StoreConfig, bridge: B) -> Result<Store<B>, StoreError> {
        let packets: Vec<MsgData> = match bridge.read() {
            Ok(packets) => Ok(packets),
            Err(db_error) => Err(StoreError::Db(db_error)),
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
                }
                None => None,
            },
        };
        for packet in packets {
            let uuid = packet.get_uuid();
            let msg_byte_size = packet.get_byte_size();
            let priority = uuid.get_priority();
            if let Some(collection) = collections.get_mut(&priority) {
                collection.byte_size += msg_byte_size;
                collection.messages.insert(uuid.clone(), msg_byte_size);
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
            collections,
            listener: None
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
        if let Some(collections) = &self.config.collections {
            if let Some(collection) = collections.get(&priority) {
                if let Some(limit) = collection.limit {
                    if msg_byte_size > limit {
                        return Err(StoreError::ExceedsCollectionLimit);
                    }
                }
            }
        }
        // get available byte size
        let mut used_byte_size: u64 = 0;
        for (col_priority, collection) in self.collections.iter() {
            if col_priority < priority {
                break;
            }
            used_byte_size += collection.byte_size;
        }
        if let Some(limit) = self.config.limit {
            let available_byte_size = limit - used_byte_size;
            if msg_byte_size > available_byte_size {
                return Err(StoreError::LacksPriority);
            }
        }
        // validate that the new collection size does not exceed collection limit
        if let Some(collection) = self.collections.get_mut(priority) {
            let proposed_byte_size = collection.byte_size + msg_byte_size;
            if let Some(collection_configs) = &self.config.collections {
                if let Some(collection_config) = collection_configs.get(&priority) {
                    if let Some(limit) = collection_config.limit {
                        if proposed_byte_size > limit {
                            let excess_bytes = proposed_byte_size - limit;
                            let mut bytes_removed: u64 = 0;
                            let mut msgs_removed: Vec<(Rc<Uuid>, ByteSize)> = vec![];
                            for (uuid, byte_size) in collection.messages.iter() {
                                bytes_removed += byte_size;
                                msgs_removed.push((uuid.clone(), *byte_size));
                                if bytes_removed >= excess_bytes {
                                    break;
                                }
                            }
                            // remove id's from collection
                            for msg in msgs_removed {
                                if let Err(error) = self.bridge.del(msg.0.clone()) {
                                    return Err(StoreError::Db(error))
                                }
                                collection.messages.remove(&msg.0);
                                collection.byte_size -= msg.1;
                                self.byte_size -= msg.1;
                                if let Some(event_handle) = &self.listener {
                                    event_handle(EventMsg::MsgBurned(msg.0, *priority, msg.1));
                                }
                            }
                        }
                    }
                }
            }
        }
        // validate that the store does not exceed the store limit
        if let Some(limit) = self.config.limit {
            let proposed_byte_size = self.byte_size + msg_byte_size;
            if proposed_byte_size > limit {
                let excess_bytes = proposed_byte_size - limit;
                let mut bytes_removed: u64 = 0;
                let mut collections_to_be_removed: Vec<Priority> = vec![];
                for (priority, collection) in self.collections.iter_mut().rev() {
                    let mut bytes_removed_from_col: u64 = 0;
                    let mut uuids_removed: Vec<(Rc<Uuid>, ByteSize)> = vec![];
                    for (uuid, byte_size) in collection.messages.iter() {
                        bytes_removed += byte_size;
                        bytes_removed_from_col += byte_size;
                        uuids_removed.push((uuid.clone(), *byte_size));
                        if bytes_removed >= excess_bytes {
                            break;
                        }
                    }
                    for msg in uuids_removed {
                        if let Err(error) = self.bridge.del(msg.0.clone()) {
                            return Err(StoreError::Db(error))
                        }
                        collection.messages.remove(&msg.0);
                        collection.byte_size -= msg.1;
                        self.byte_size -= msg.1;
                        if let Some(event_handle) = &self.listener {
                            event_handle(EventMsg::MsgBurned(msg.0.clone(), *priority, msg.1));
                        }
                    }
                    if collection.messages.len() == 0 {
                        collections_to_be_removed.push(*priority);
                    }
                    if bytes_removed >= excess_bytes {
                        break;
                    }
                }
                for priority in collections_to_be_removed {
                    self.collections.remove(&priority);
                }
            }
        }
        if let Err(error) = self.bridge.put(uuid.clone(), &msg) {
            return Err(StoreError::Db(error))
        }
        // add msg data to collection
        if let Some(collection) = self.collections.get_mut(priority) {
            collection.messages.insert(uuid.clone(), msg_byte_size);
            collection.byte_size += msg_byte_size;
        } else {
            let mut collection;
            if let Some(collection_configs) = &self.config.collections {
                if let Some(config) = collection_configs.get(&priority) {
                    collection = Collection::new(&config);
                } else {
                    collection = Collection::new(&CollectionConfig::new(*priority, None));
                }
            } else {
                collection = Collection::new(&CollectionConfig::new(*priority, None));
            }
            collection.byte_size += msg_byte_size;
            collection.messages.insert(uuid.clone(), msg_byte_size);
            self.collections.insert(*priority, collection);
        }
        self.byte_size += msg_byte_size;
        if let Some(event_handle) = &self.listener {
            event_handle(EventMsg::MsgAdded);
        }
        Ok(())
    }

    pub fn del(&mut self, id: &str) -> Result<(), StoreError> {
        let uuid = Rc::new(Uuid::from_string(id));
        let priority = uuid.get_priority();
        let mut remove_collection = false;
        if let Some(collection) = self.collections.get_mut(&priority) {
            if collection.messages.contains_key(&uuid) {
                if let Err(error) = self.bridge.del(uuid.clone()) {
                    return Err(StoreError::Db(error))
                }
            }
            let bytes_removed = { 
                if let Some(bytes_removed) = collection.messages.remove(&uuid) {
                    bytes_removed
                } else {
                    0
                }
            };
            collection.byte_size -= bytes_removed;
            self.byte_size -= bytes_removed;
            if collection.messages.len() == 0 {
                remove_collection = true;
            }
        }
        if remove_collection {
            self.collections.remove(&priority);
        }
        if let Some(event_handle) = &self.listener {
            event_handle(EventMsg::MsgDeleted);
        }
        Ok(())
    }

    pub fn get(&self) -> Result<Option<Packet>, StoreError> {
        if let Some((_prioriy, col_ref)) = self.collections.iter().rev().next() {
            if let Some((uuid, _byte_size)) = col_ref.messages.iter().next() {
                let id = uuid.to_string();
                return match self.bridge.get(uuid.clone()) {
                    Ok(msg_option) => match msg_option {
                        Some(msg) => Ok(Some(Packet { id, msg })),
                        None => Ok(None),
                    },
                    Err(error) => Err(StoreError::Db(error)),
                };
            } else {
                return Err(StoreError::OutOfSync);
            }
        }
        Ok(None)
    }

    pub fn dump(&mut self) -> Result<Option<Packet>, StoreError> {
        let mut priority_removed: Option<Priority> = None;
        let mut uuid_option: Option<Rc<Uuid>> = None;
        let uuid_removed: Rc<Uuid>;
        let mut bytes_removed = 0;
        let mut packet: Option<Packet> = None;
        if let Some((priority, collection)) = self.collections.iter_mut().rev().next() {
            if let Some((uuid, byte_size)) = collection.messages.iter().next() {
                let id = uuid.to_string();
                uuid_option = Some(uuid.clone());
                uuid_removed = uuid.clone();
                match self.bridge.get(uuid.clone()) {
                    Ok(msg_option) => match msg_option {
                        Some(msg) => {
                            match self.bridge.del(uuid.clone()) {
                                Ok(_) => {
                                    bytes_removed = *byte_size;
                                    packet = Some(Packet { id, msg });
                                },
                                Err(error) => {
                                   return Err(StoreError::Db(error))
                                }
                            }
                        } ,
                        None => {
                            // packet = None;
                        }
                    },
                    Err(error) => { 
                        return Err(StoreError::Db(error))
                    }
                };
            } else {
                return Err(StoreError::OutOfSync);
            }
            collection.messages.remove(&uuid_removed);
            collection.byte_size -= bytes_removed;
            self.byte_size -= bytes_removed;
            if collection.messages.len() == 0 {
               priority_removed = Some(*priority);
            }
        }
        if let Some(priority) = priority_removed {
            self.collections.remove(&priority);
        }
        if let Some(event_handle) = &self.listener {
            if let Some(uuid) = uuid_option {
                event_handle(EventMsg::MsgDumped(uuid.clone(), bytes_removed));
            }
        }
        Ok(packet)
    }

    pub fn get_collections(&self) -> Vec<u64> {
        let mut list: Vec<u64> = vec![];
        for priority in self.collections.keys() {
            list.push(*priority);
        }
        list
    }

    pub fn get_collection_msg_count(&self, priority: &u64) -> Option<(u64, u64)> {
        match self.collections.get(priority) {
            Some(collection) => {
                let byte_size = collection.byte_size;
                let msg_count = collection.messages.len() as u64;
                Some((byte_size, msg_count))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::db_bridge::LevelDbBridge;

    use std::collections::HashMap;
    use super::*;
    use enum_as_inner::EnumAsInner;
    use laboratory::{Suite, NullState, LabResult, describe, expect};
    use tempdir::TempDir;

    #[derive(EnumAsInner)]
     enum State {
         Count(u32),
         Store(Store<LevelDbBridge>)
     } 

    #[test]
    fn laboratory() -> LabResult {
        describe("msg-store", |suite| {
            suite.before_all(|state| {
                state.insert("/count", State::Count(0));
            }).before_each(|state| {
                let count = state
                    .get("/count").unwrap()
                    .as_count().unwrap();
                let config = StoreConfig {
                    dir: TempDir::new(&format!("{}", count)).unwrap().into_path(),
                    limit: None,
                    collections: None
                };
                let db = LevelDbBridge::new(&config.dir).unwrap();
                let store = Store::new(config, db).unwrap();
                state.insert("/store", State::Store(store));
            }).after_each(|state| {
                // let mut state = state_map.get_mut("/count").unwrap(); 
                // let count = state.
                let count = state
                    .get_mut("/count").unwrap()
                    .as_count_mut().unwrap();
                *count += 1; 
            }).after_all(|state| {
                let count = state.get("/count").unwrap().as_count().unwrap();
                println!("count = {}", count);
            }).it("should add one msg to a collection and update byte_sizes", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                let msg = "0123456789";
                store.add(&0, &msg.as_bytes().to_vec()).unwrap();
                let collection = store.collections.get(&0).unwrap();
                expect(store.byte_size).to_be(10)?;
                expect(collection.byte_size).to_be(10)?;
                expect(collection.messages.len()).to_equal(1)
            }).it("should remove collection and total byte size of store", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                let msg = "0123456789";
                store.add(&0, &msg.as_bytes().to_vec()).unwrap();
                let id_str = {
                    let collection = store.collections.get(&0).unwrap();
                    expect(store.byte_size).to_be(10)?;
                    expect(collection.byte_size).to_be(10)?;
                    expect(collection.messages.len()).to_equal(1)?;
                    let msg = collection.messages.iter().next().unwrap(); 
                    msg.0.to_string()
                };
                store.del(&id_str).unwrap();
                let collection = store.collections.get(&0);
                expect(store.byte_size).to_be(0)?;
                expect(collection.is_none()).to_be(true)
            }).it("should not remove collection but byte size of collection & store", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                let msg = "0123456789";
                store.add(&0, &msg.as_bytes().to_vec()).unwrap();
                store.add(&0, &msg.as_bytes().to_vec()).unwrap();
                let id_str = {
                    let collection = store.collections.get(&0).unwrap();
                    expect(store.byte_size).to_be(20)?;
                    expect(collection.byte_size).to_be(20)?;
                    expect(collection.messages.len()).to_equal(2)?;
                    let msg = collection.messages.iter().next().unwrap(); 
                    msg.0.to_string()
                };
                store.del(&id_str).unwrap();
                let collection = store.collections.get(&0).unwrap();
                expect(store.byte_size).to_be(10)?;
                expect(collection.byte_size).to_be(10)?;
                expect(collection.messages.len()).to_be(1)
            }).it("should remove a lower priority and add a higher priority msg", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                store.config.limit = Some(10);
                let msg = "0123456789";
                store.add(&0, &msg.as_bytes().to_vec()).unwrap();
                store.add(&1, &msg.as_bytes().to_vec()).unwrap();
                let collection_1 = store.collections.get(&0);
                let collection_2 = store.collections.get(&1).unwrap(); 
                expect(store.byte_size).to_be(10)?;
                expect(collection_2.byte_size).to_be(10)?;
                expect(collection_2.messages.len()).to_equal(1)?;
                expect(collection_1.is_none()).to_be(true)
            }).it("should get highest priority msg", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                let msg_1 = "0123456789";
                let msg_2 = "0987654321";
                store.add(&0, &msg_1.as_bytes().to_vec()).unwrap();
                store.add(&1, &msg_2.as_bytes().to_vec()).unwrap();
                let collection = store.collections.get(&1).unwrap();
                let data = collection.messages.iter().next().unwrap();
                let id_str = data.0.to_string();
                let packet = store.get().unwrap().unwrap();
                expect(packet.id).to_be(id_str)?;
                expect(String::from_utf8(packet.msg).unwrap().as_str()).to_be(msg_2)
            }).it("should reject messages that are to large", |spec| {
                let mut state = spec.state
                    .borrow_mut();
                let store = state
                    .get_mut("/store")
                    .unwrap()
                    .as_store_mut()
                    .unwrap();
                store.config.limit = Some(20);
                {
                    let collection_config = CollectionConfig {
                        priority: 0,
                        limit: Some(10)
                    };
                    let mut collections = HashMap::new();
                    collections.insert(0, collection_config);
                    store.config.collections = Some(collections);
                }
                let msg_1 = "123456789012345678901".as_bytes().to_vec();
                let msg_2 = "12345678901".as_bytes().to_vec();
                let msg_3 = "1234567890".as_bytes().to_vec();
                expect(store.add(&0, &msg_1).is_err()).to_be(true)?;
                expect(store.add(&0, &msg_2).is_err()).to_be(true)?;
                expect(store.add(&0, &msg_3).is_ok()).to_be(true)
            });
        }).run()
    }
}

