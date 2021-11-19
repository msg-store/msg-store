use crate::{
    errors::{
        Error
    },
    Keeper,
    uuid::{
        UuidManager,
        Uuid
    }
};

use std::collections::BTreeMap;

enum PruneBy {
    Group,
    Store
}

pub struct StoreDefaults {
    pub max_byte_size: Option<i32>
}

#[derive(Debug, Clone, Copy)]
pub struct GroupDefaults {
    pub max_byte_size: Option<i32>,
}

pub struct Group {
    pub max_byte_size: Option<i32>,
    pub byte_size: i32,
    pub msgs_map: BTreeMap<Uuid, i32>,
}
impl Group {
    pub fn new(max_byte_size: Option<i32>) -> Group {
        Group { 
            max_byte_size,
            byte_size: 0, 
            msgs_map: BTreeMap::new() 
        }
    }
    pub fn update_from_config(&mut self, defaults: GroupDefaults) {
        self.max_byte_size = defaults.max_byte_size;
    }
}

struct RemovedMsgs {
    priority: i32,
    msgs: Vec<Uuid>
}
impl RemovedMsgs {
    pub fn new(priority: i32) -> RemovedMsgs {
        RemovedMsgs {
            priority,
            msgs: vec![]
        }
    }
    pub fn add(&mut self, uuid: Uuid) {
        self.msgs.push(uuid);
    }
}

pub struct Package {
    pub uuid: Uuid,
    pub priority: i32,
    pub msg: String,
    pub byte_size: i32
}

#[derive(Debug)]
pub struct StoredPacket {
    pub uuid: Uuid,
    pub msg: String
}

pub struct Packet {
    priority: i32,
    msg: String
}
impl Packet {
    pub fn new(priority: i32, msg: String) -> Packet {
        Packet { priority, msg }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PacketMetaData {
    pub uuid: Uuid,
    pub priority: i32,
    pub byte_size: i32
}


/// The base unit which stores information about inserted messages and priority groups
/// to determine which messages should be forwarded or burned first.
/// 
/// The store can contain 2,147,483,647 priorities.
/// Messages are forwarded on a highest priority then oldest status basis.
/// Messages are burned/pruned on a lowest priority then oldest status basis.
/// Messages are only burned once the store has reached the max bytesize limit.
/// The store as a whole contains a max bytesize limit option as does each individule priority
/// group. For example, a developer can limit the size of the store to 1,000 bytes, while restricting
/// priority group 1 to only 500 bytes, and leave higher priorities free with no restriction (except that of the store.)
/// 
/// The store keeps track of basic statistics such as counting the messages that have been inserted, deleted, or burned.
/// Messages that have been deleted have been so on instructions of the developer using the del method.
/// Messages that have been burned have been so automatically on insert or store/group defaults update once the
/// max bytesize limit has been reached.
pub struct Store<Db: Keeper> {
    pub max_byte_size: Option<i32>,
    pub byte_size: i32,
    pub group_defaults: BTreeMap<i32, GroupDefaults>,
    pub uuid_manager: UuidManager,
    pub db: Db,
    pub id_to_group_map: BTreeMap<Uuid, i32>,
    pub groups_map: BTreeMap<i32, Group>,
    pub msgs_inserted: i32,
    pub msgs_deleted: i32,
    pub msgs_burned: i32
}

impl<Db: Keeper> Store<Db> {

    pub fn open(db: Db) -> Result<Store<Db>, Error> {        
        let mut store = Store {
            max_byte_size: None,
            byte_size: 0,
            group_defaults: BTreeMap::new(),
            uuid_manager: UuidManager::default(),
            db,
            id_to_group_map: BTreeMap::new(),
            groups_map: BTreeMap::new(),
            msgs_inserted: 0,
            msgs_deleted: 0,
            msgs_burned: 0
        };
        let mut data = match store.db.fetch() {
            Ok(data) => Ok(data),
            Err(db_error) => Err(Error::DbError(db_error))
        }?;
        data.sort();
        data.iter().map(|data| -> Result<(), Error> {
            store.insert_data(&data)
        }).collect::<Result<(), Error>>()?;
        Ok(store)
    }

    /// A method for reseting the inserted messages count
    pub fn clear_msgs_inserted_count(&mut self) {
        self.msgs_inserted = 0;
    }

    fn inc_msgs_inserted_count(&mut self) {
        if self.msgs_inserted == i32::MAX {
            self.msgs_inserted = 1;
        } else {
            self.msgs_inserted += 1;
        }
    }

    /// A method for reseting the burned messages count
    pub fn clear_msgs_burned_count(&mut self) {
        self.msgs_burned = 0;
    }

    fn inc_msgs_burned_count(&mut self) {
        if self.msgs_burned == i32::MAX {
            self.msgs_burned = 1;
        } else {
            self.msgs_burned += 1;
        }
    }

    /// A method for reseting the deleted messages count
    pub fn clear_msgs_deleted_count(&mut self) {
        self.msgs_deleted = 0;
    }

    fn inc_msgs_deleted(&mut self, msgs_deleted: i32) {
        let diff = (i32::MAX - self.msgs_deleted) - msgs_deleted;
        if diff < 0 {
            self.msgs_deleted = diff.abs();
        } else {
            self.msgs_deleted += msgs_deleted;
        }
    }

    fn msg_excedes_max_byte_size(byte_size: &i32, max_byte_size: &i32, msg_byte_size: &i32) -> bool {
        &(byte_size + msg_byte_size) > max_byte_size
    }

    fn remove_msg(&mut self, uuid: &Uuid, group: &mut Group) -> Result<(), Error> {
        let byte_size = match group.msgs_map.remove(&uuid) {
            Some(byte_size) => Ok(byte_size),
            None => Err(Error::SyncError)
        }?;
        self.id_to_group_map.remove(&uuid);
        self.byte_size -= byte_size;
        group.byte_size -= byte_size;
        match self.db.del(&uuid) {
            Ok(()) => Ok(()),
            Err(db_error) => Err(Error::DbError(db_error))
        }?;
        self.inc_msgs_burned_count();
        Ok(())
    }
    
    fn get_group(&mut self, priority: i32) -> Group {
        match self.groups_map.remove(&priority) {
            Some(group) => group,
            None => {
                let max_byte_size = match self.group_defaults.get(&priority) {
                    Some(defaults) => defaults.max_byte_size.clone(),
                    None => None
                };
                Group::new(max_byte_size)
            }
        }
    }

    fn check_msg_size_agains_store(&self, msg_byte_size: i32) -> Result<(), Error> {
        if let Some(store_max_byte_size) = self.max_byte_size {
            if msg_byte_size > store_max_byte_size {
                return Err(Error::ExceedesStoreMax)
            }
        }
        Ok(())
    }

    fn check_msg_size_against_group(&mut self, group: Group, msg_priority: i32, msg_byte_size: i32) -> Result<Group, Error> {
        // check if the msg is too large for the target group
        if let Some(group_max_byte_size) = &group.max_byte_size {
            if &msg_byte_size > group_max_byte_size {
                self.groups_map.insert(msg_priority, group);
                return Err(Error::ExceedesGroupMax);
            }
        }

        // get the total byte count of all msgs that are higher priority
        // in order to know how free bytes are remaining for the new message
        let higher_priority_msg_total = {
            let mut total = 0;
            for (priority, group) in self.groups_map.iter().rev() {
                if &msg_priority > priority {
                    break;
                }
                total += group.byte_size;
            }
            total
        };

        // check if there is enough free space for the message
        if let Some(store_max_byte_size) = self.max_byte_size {
            if Self::msg_excedes_max_byte_size(&higher_priority_msg_total, &store_max_byte_size, &msg_byte_size) {
                self.groups_map.insert(msg_priority, group);
                return Err(Error::LacksPriority);
            }
        }
        Ok(group)
    }

    fn prune_group(&mut self, group: &mut Group, msg_byte_size: i32, prune_type: PruneBy) -> Result<(), Error> {
        let (byte_size, max_byte_size) = match prune_type {
            PruneBy::Group => (group.byte_size, group.max_byte_size),
            PruneBy::Store => (self.byte_size, self.max_byte_size)
        };
        if let Some(max_byte_size) = &max_byte_size {            
            if Self::msg_excedes_max_byte_size(&byte_size, max_byte_size, &msg_byte_size) {
                // prune group
                let mut removed_msgs = vec![];
                let mut bytes_removed = 0;
                for (uuid, group_msg_byte_size) in group.msgs_map.iter() {
                    if !Self::msg_excedes_max_byte_size(&(byte_size - bytes_removed), max_byte_size, &msg_byte_size) {
                        break;
                    }
                    bytes_removed += group_msg_byte_size;
                    removed_msgs.push(uuid.clone());
                }
                for uuid in removed_msgs.iter() {
                    self.remove_msg(&uuid, group)?;
                }
            }
        }
        Ok(())
    }

    fn prune_store(&mut self, group: Option<&mut Group>, msg_priority: i32, msg_byte_size: i32) -> Result<(), Error> {
        if let Some(store_max_byte_size) = self.max_byte_size.clone() {
            if Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                let mut groups_removed = vec![];
                let mut all_removed_msgs = vec![];
                let mut bytes_removed = 0;
                'groups: for (priority, group) in self.groups_map.iter_mut() {
                    if &msg_priority < priority {
                        break 'groups;
                    }
                    if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                        break 'groups;
                    }
                    let mut removed_msgs = RemovedMsgs::new(*priority);
                    'messages: for (uuid, group_msg_byte_size) in group.msgs_map.iter() {
                        if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                            break 'messages;
                        }
                        bytes_removed += group_msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }
                    if group.byte_size == 0 {
                        groups_removed.push(*priority);
                    }
                    all_removed_msgs.push(removed_msgs);
                }
                // get groups of msgs that where removed
                for group_data in all_removed_msgs {
                    let mut group = match self.groups_map.remove(&group_data.priority) {
                        Some(group) => Ok(group),
                        None => Err(Error::SyncError)
                    }?;
                    for uuid in group_data.msgs {
                        self.remove_msg(&uuid, &mut group)?;
                    }
                    self.groups_map.insert(group_data.priority, group);
                }
                for priority in groups_removed {
                    self.groups_map.remove(&priority);
                }

                // prune group again
                if let Some(group) = group {
                    self.prune_group(group, msg_byte_size, PruneBy::Store)?;
                }
            }            
        }
        Ok(())
    }

    fn insert_msg(&mut self, mut group: Group, uuid: Uuid, priority: i32, msg_byte_size: i32) {
        self.byte_size += msg_byte_size;                                          // increase store byte size
        self.id_to_group_map.insert(uuid.clone(), priority);            // insert the uuid into the uuid->priority map
        group.byte_size += msg_byte_size;                                         // increase the group byte size
        group.msgs_map.insert(uuid.clone(), msg_byte_size);             // insert the uuid into the uuid->byte size map
        self.groups_map.insert(priority, group);
    }

    fn insert_data(&mut self, data: &PacketMetaData) -> Result<(), Error> {

        // check if the msg is too large for the store
        self.check_msg_size_agains_store(data.byte_size)?;

        // check if the target group exists
        // create if id does not
        let group = self.get_group(data.priority);

        // check if the msg is too large for the target group
        let mut group = self.check_msg_size_against_group(group, data.priority, data.byte_size)?;

        // prune group if needed
        self.prune_group(&mut group, data.byte_size, PruneBy::Group)?;

        // prune store
        self.prune_store(Some(&mut group), data.priority, data.byte_size)?;

        // insert msg
        self.insert_msg(group, data.uuid, data.priority, data.byte_size);

        Ok(())
    }

    fn get_stored_packet(&mut self, uuid: Uuid) -> Result<Option<StoredPacket>, Error> {
        let possible_msg = match self.db.get(&uuid) {
            Ok(data) => Ok(data),
            Err(db_error) => Err(Error::DbError(db_error))
        }?;
        if let Some(msg) = possible_msg {
            Ok(Some(StoredPacket {
                uuid,
                msg
            }))
        } else {
            Ok(None)
        }
    }

    /// Adds a msg to the store
    /// 
    /// The message itself is written to disk as well as metadata about the message
    /// such as its bytesize and priority. The priority and bytesize will
    /// also be held in memory for quick access. A unique uuid will be returned
    /// on success.
    /// 
    /// The store's inserted message count will also be incremented by one.
    /// 
    /// # Errors
    /// The method will return an error when:
    /// * the message byte size exceeds the store's max byte size limit.
    /// * the message byte size exceeds the priority group's max byte size limit.
    /// * the message byte size does not exceed either the store's or group's max limit, but
    ///    where the the store does not have enough space for it after accounting for
    ///    higher priority messages i.e., higher priority messages will not be removed to make
    ///    space for lower priority ones.
    /// * there is an error while reading or writing to disk.
    /// 
    /// The error wiil be returned as a string.
    /// 
    /// # Examples
    /// ```
    /// use msg_store::{ Packet, open };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).unwrap();
    /// 
    /// ```
    /// 
    pub fn add(&mut self, packet: &Packet) -> Result<Uuid, Error> {

        let msg_byte_size = packet.msg.len() as i32;

        // check if the msg is too large for the store
        self.check_msg_size_agains_store(msg_byte_size)?;

        // check if the target group exists
        // create if id does not
        let group = self.get_group(packet.priority);

        // check if the msg is too large for the target group
        let mut group = self.check_msg_size_against_group(group, packet.priority, msg_byte_size)?;

        // prune group if needed
        self.prune_group(&mut group, msg_byte_size, PruneBy::Group)?;

        // prune store
        self.prune_store(Some(&mut group), packet.priority, msg_byte_size)?;

        // insert msg
        let uuid = self.uuid_manager.next();                                 // get uuid
        self.insert_msg(group, uuid, packet.priority, msg_byte_size);
        
        let package = Package {
            uuid,
            priority: packet.priority,
            msg: packet.msg.clone(),
            byte_size: msg_byte_size
        };
        match self.db.add(&package) {
            Ok(_) => Ok(()),
            Err(db_error) => Err(Error::DbError(db_error))
        }?;
        self.inc_msgs_inserted_count();

        Ok(uuid)
        
    }
    
    /// Deletes a message from the store
    /// 
    /// A message will be removed from the store and disk once given the
    /// the message's uuid number.
    /// 
    /// # Errors
    /// An error will be returned if there is an issue removing the message from disk.
    /// 
    /// # Examples
    /// ```
    /// use msg_store::{ Packet, open };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).unwrap();
    /// store.del(&uuid).unwrap();
    /// 
    /// ```
    pub fn del(&mut self, uuid: &Uuid) -> Result<(), Error> {
        let mut remove_group = false;
        let priority = match self.id_to_group_map.get(&uuid) {
            Some(priority) => priority,
            None => {
                return Ok(());
            }
        };
        let mut group = match self.groups_map.get_mut(&priority) {
            Some(group) => group,
            None => {
                return Ok(());
            }
        };
        let bytes_removed = match group.msgs_map.remove(&uuid) {
            Some(bytes_removed) => bytes_removed,
            None => {
                return Ok(());
            }
        };
        group.byte_size -= bytes_removed;
        self.byte_size -= bytes_removed;
        if group.msgs_map.is_empty() {
            remove_group = true;
        }
        if remove_group {
            self.groups_map.remove(&priority);
        }
        self.id_to_group_map.remove(&uuid);
        match self.db.del(&uuid) {
            Ok(_) => Ok(()),
            Err(db_error) => Err(Error::DbError(db_error))
        }?;
        self.inc_msgs_deleted(1);
        Ok(())
    }

    /// Gets a message from the store, either the next in line, the next in a specified priority group, or a specific message
    /// specified by the uuid option.
    /// 
    /// If the uuid option is present, it will search for that uuid only. If the priority option is present, it will retrieve the next
    /// message in line for that priority only. If neither options are present, the store will retrieve the next message in line store wide. 
    /// If no message is found, None is returned.
    /// 
    /// # Error
    /// The method will panic when encountering a disk error.
    /// 
    /// # Examples
    /// ```
    /// use msg_store::{ Packet, open };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).unwrap();
    /// let my_message = store.get(Some(uuid), None).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(None, Some(1)).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(None, None).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// ```
    pub fn get(&mut self, uuid: Option<Uuid>, priority: Option<i32>) -> Result<Option<StoredPacket>, Error> {
        if let Some(uuid) = uuid {
            self.get_stored_packet(uuid)
        } else {
            if let Some(priority) = priority {
                let uuid = {
                    if let Some(group) = self.groups_map.get(&priority) {
                        if let Some(uuid) = group.msgs_map.keys().next() {
                            Some(uuid.clone())
                            
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(uuid) = uuid {
                    self.get_stored_packet(uuid)
                } else {
                    Ok(None)
                }                
            } else {
                if let Some(group) = self.groups_map.values().rev().next() {
                    let uuid = {
                        if let Some(uuid) = group.msgs_map.keys().next() {
                            Ok(uuid.clone())
                        } else {
                            Err(Error::SyncError)
                        }
                    }?;
                    self.get_stored_packet(uuid)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Updates the defaults for a priority group
    /// 
    /// The method takes a GroupDefaults struct which contains a member: max_byte_size.
    /// The max_byte_size member type is Option<i32>. This method will auto prune the group
    /// if the group's current bytesize is greater than the new max bytesize default.
    /// 
    /// # Errors
    /// The method will panic if the database encounters an error
    /// 
    /// # Example
    /// ```
    /// use msg_store::{
    ///     Packet,
    ///     store::GroupDefaults,
    ///     open
    /// };
    /// 
    /// let mut store = open();
    /// store.add(&Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(&Packet::new(1, "bar".to_string())).unwrap();
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(3) });
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_group_defaults(&mut self, priority: i32, defaults: &GroupDefaults) -> Result<(), Error> {
        self.group_defaults.insert(priority, defaults.clone());
        if let Some(mut group) = self.groups_map.remove(&priority) {
            group.update_from_config(defaults.clone());
            self.prune_group(&mut group, 0, PruneBy::Group)?;
            self.groups_map.insert(priority, group);
        }
        Ok(())
    }

    /// Removes the defaults for a priority group
    /// 
    /// # Example
    /// ```
    /// use msg_store::{
    ///     Packet,
    ///     store::GroupDefaults,
    ///     open
    /// };
    /// 
    /// let mut store = open();
    /// store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(6) });
    /// store.add(&Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(&Packet::new(1, "bar".to_string())).unwrap();
    /// 
    /// let group_1 = store.groups_map.get(&1).expect("Could not find group");
    /// assert_eq!(Some(6), group_1.max_byte_size);
    /// 
    /// // Now for the removal of the defaults
    /// store.delete_group_defaults(1);
    /// 
    /// let group_1 = store.groups_map.get(&1).expect("Could not find group");
    /// 
    /// assert_eq!(None, group_1.max_byte_size);
    /// assert!(store.group_defaults.get(&1).is_none()); 
    /// 
    /// ```
    pub fn delete_group_defaults(&mut self, priority: i32) {
        self.group_defaults.remove(&priority);
        if let Some(group) = self.groups_map.get_mut(&priority) {
            group.max_byte_size = None;
        }
    }

    /// Updates the defaults for the store
    /// 
    /// The method takes a StoreDefaults struct which contains a member: max_byte_size.
    /// The max_byte_size member type is Option<i32>. This method will auto prune the store
    /// if the store's current bytesize is greater than the new max bytesize default.
    /// 
    /// # Errors
    /// The method will panic if the database encounters an error
    /// 
    /// # Example
    /// ```
    /// use msg_store::{
    ///     Packet,
    ///     store::StoreDefaults,
    ///     open
    /// };
    /// 
    /// let mut store = open();
    /// store.add(&Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(&Packet::new(1, "bar".to_string())).unwrap();
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(3) }).unwrap();
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_store_defaults(&mut self, defaults: &StoreDefaults) -> Result<(), Error> {
        self.max_byte_size = defaults.max_byte_size;
        self.prune_store(None, i32::MAX, 0)
    }

}
