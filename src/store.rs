use crate::{
    errors::Error,
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
pub enum Deleted {
    True,
    False
}

pub struct StoreDefaults {
    pub max_byte_size: Option<u32>
}

#[derive(Debug, Clone, Copy)]
pub struct GroupDefaults {
    pub max_byte_size: Option<u32>,
}

pub struct Group {
    pub max_byte_size: Option<u32>,
    pub byte_size: u32,
    pub msgs_map: BTreeMap<Uuid, u32>,
}
impl Group {
    pub fn new(max_byte_size: Option<u32>) -> Group {
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
    priority: u32,
    msgs: Vec<Uuid>
}
impl RemovedMsgs {
    pub fn new(priority: u32) -> RemovedMsgs {
        RemovedMsgs {
            priority,
            msgs: vec![]
        }
    }
    pub fn add(&mut self, uuid: Uuid) {
        self.msgs.push(uuid);
    }
}

// pub struct Package {
//     pub uuid: Uuid,
//     pub priority: u32,
//     pub msg: String,
//     pub byte_size: u32
// }

#[derive(Debug, Clone)]
pub struct StoredPacket {
    pub uuid: Uuid,
    pub msg: String
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct PacketMetaData {
    pub uuid: Uuid,
    pub priority: u32,
    pub byte_size: u32
}

pub struct AddResult {
    pub uuid: Uuid,
    pub bytes_removed: u32,
    pub groups_removed: Vec<u32>,
    pub msgs_removed: Vec<Uuid>
} 

/// The base unit which stores information about inserted messages and priority groups
/// to determine which messages should be forwarded or burned first.
/// 
/// The store can contain 4,294,967,295 priorities.
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
pub struct Store {
    pub max_byte_size: Option<u32>,
    pub byte_size: u32,
    pub group_defaults: BTreeMap<u32, GroupDefaults>,
    pub uuid_manager: UuidManager,
    pub id_to_group_map: BTreeMap<Uuid, u32>,
    pub groups_map: BTreeMap<u32, Group>,
    pub msgs_inserted: u32,
    pub msgs_deleted: u32,
    pub msgs_pruned: u32
}

impl Store {

    pub fn open() -> Result<Store, Error> {
        let store = Store {
            max_byte_size: None,
            byte_size: 0,
            group_defaults: BTreeMap::new(),
            uuid_manager: UuidManager::default(),
            id_to_group_map: BTreeMap::new(),
            groups_map: BTreeMap::new(),
            msgs_inserted: 0,
            msgs_deleted: 0,
            msgs_pruned: 0
        };
        Ok(store)
    }

    /// A method for reseting the inserted messages count
    pub fn clear_msgs_inserted_count(&mut self) {
        self.msgs_inserted = 0;
    }

    fn inc_msgs_inserted_count(&mut self) {
        if self.msgs_inserted == u32::MAX {
            self.msgs_inserted = 1;
        } else {
            self.msgs_inserted += 1;
        }
    }

    /// A method for reseting the burned messages count
    pub fn clear_msgs_pruned_count(&mut self) {
        self.msgs_pruned = 0;
    }

    fn inc_msgs_pruned_count(&mut self) {
        if self.msgs_pruned == u32::MAX {
            self.msgs_pruned = 1;
        } else {
            self.msgs_pruned += 1;
        }
    }

    /// A method for reseting the deleted messages count
    pub fn clear_msgs_deleted_count(&mut self) {
        self.msgs_deleted = 0;
    }

    fn inc_msgs_deleted(&mut self) {
        if self.msgs_deleted == u32::MAX {
            self.msgs_deleted = 1;
        } else {
            self.msgs_deleted += 1;
        }
    }

    fn msg_excedes_max_byte_size(byte_size: &u32, max_byte_size: &u32, msg_byte_size: &u32) -> bool {
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
        self.inc_msgs_pruned_count();
        Ok(())
    }
    
    fn get_group(&mut self, priority: u32) -> Group {
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

    fn check_msg_size_agains_store(&self, msg_byte_size: u32) -> Result<(), Error> {
        if let Some(store_max_byte_size) = self.max_byte_size {
            if msg_byte_size > store_max_byte_size {
                return Err(Error::ExceedesStoreMax)
            }
        }
        Ok(())
    }

    fn check_msg_size_against_group(&mut self, group: Group, msg_priority: u32, msg_byte_size: u32) -> Result<Group, Error> {
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

    fn prune_group(&mut self, group: &mut Group, msg_byte_size: u32, prune_type: PruneBy) -> Result<(u32, Vec<Uuid>), Error> {
        let (byte_size, max_byte_size) = match prune_type {
            PruneBy::Group => (group.byte_size, group.max_byte_size),
            PruneBy::Store => (self.byte_size, self.max_byte_size)
        };
        let mut removed_msgs = vec![];
        let mut bytes_removed = 0;
        if let Some(max_byte_size) = &max_byte_size {            
            if Self::msg_excedes_max_byte_size(&byte_size, max_byte_size, &msg_byte_size) {
                // prune group
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
        Ok((bytes_removed, removed_msgs))
    }

    fn prune_store(&mut self, group: Option<&mut Group>, msg_priority: u32, msg_byte_size: u32) -> Result<(u32, Vec<u32>, Vec<Uuid>), Error> {
        let mut groups_removed = vec![];
        let mut all_removed_msgs = vec![];
        let mut bytes_removed = 0;
        {
            if let Some(store_max_byte_size) = self.max_byte_size.clone() {
                if Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
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
                    for group_data in &all_removed_msgs {
                        let mut group = match self.groups_map.remove(&group_data.priority) {
                            Some(group) => Ok(group),
                            None => Err(Error::SyncError)
                        }?;
                        for uuid in &group_data.msgs {
                            self.remove_msg(&uuid, &mut group)?;
                        }
                        self.groups_map.insert(group_data.priority, group);
                    }
                    for priority in &groups_removed {
                        self.groups_map.remove(&priority);
                    }
    
                    // prune group again
                    if let Some(group) = group {
                        self.prune_group(group, msg_byte_size, PruneBy::Store)?;
                    }
                }            
            }
        }
        
        let msgs_removed: Vec<Uuid> = all_removed_msgs.into_iter().map(|removed_msgs| removed_msgs.msgs).flatten().collect();
        Ok((bytes_removed, groups_removed, msgs_removed))
    }

    fn insert_msg(&mut self, mut group: Group, uuid: Uuid, priority: u32, msg_byte_size: u32) {
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
    /// * the database implimentation encounters an error. Please read the database plugin's documentation for details.
    /// 
    /// The error wiil be returned as a string.
    /// 
    /// # Example
    /// ```
    /// use msg_store::{ Packet, open };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(Packet::new(1, "my message".to_string())).unwrap();
    /// 
    /// ```
    /// 
    pub fn add(&mut self, priority: u32, msg: String) -> Result<AddResult, Error> {

        let msg_byte_size = msg.len() as u32;

        // check if the msg is too large for the store
        self.check_msg_size_agains_store(msg_byte_size)?;

        // check if the target group exists
        // create if id does not
        let group = self.get_group(priority);

        // check if the msg is too large for the target group
        let mut group = self.check_msg_size_against_group(group, priority, msg_byte_size)?;

        let mut bytes_removed = 0;
        let mut groups_removed = vec![];
        let mut msgs_removed = vec![];

        // prune group if needed
        let (bytes_removed_from_group, mut msgs_removed_from_group) = self.prune_group(&mut group, msg_byte_size, PruneBy::Group)?;

        bytes_removed += bytes_removed_from_group;
        msgs_removed.append(&mut msgs_removed_from_group);

        // prune store
        let (bytes_removed_from_groups, mut groups_removed_from_store, mut msgs_removed_from_groups) = self.prune_store(Some(&mut group), priority, msg_byte_size)?;
        bytes_removed += bytes_removed_from_groups;
        msgs_removed.append(&mut msgs_removed_from_groups);
        groups_removed.append(&mut groups_removed_from_store);

        // insert msg
        let uuid = self.uuid_manager.next();                                 // get uuid
        self.insert_msg(group, uuid, priority, msg_byte_size);
        
        self.inc_msgs_inserted_count();

        Ok(AddResult{ uuid, bytes_removed, msgs_removed, groups_removed })
        
    }
    
    /// Deletes a message from the store
    /// 
    /// A message will be removed from the store and disk once given the
    /// the message's uuid number.
    /// 
    /// The message store's msgs_deleted member will also be incremented by one.
    /// 
    /// # Errors
    /// An error will be returned if the database encounters an error, read the database plugin documention for specifics.
    /// 
    /// # Example
    /// ```
    /// use msg_store::{ Packet, open };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(Packet::new(1, "my message".to_string())).unwrap();
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
        self.inc_msgs_deleted();
        Ok(())
    }

    /// Deletes a group and its messages from the store
    /// 
    /// A group's metadata and messages will be removed from the store and disk once given the
    /// the group's priority number.
    /// 
    /// The message store's msgs_deleted member will also be incremented by one.
    /// 
    /// # Errors
    /// An error will be returned if the database encounters an error, read the database plugin documention for specifics.
    /// 
    /// # Example
    /// ```
    /// use msg_store::{ Packet, open, GetOptions };
    /// 
    /// let mut store = open();
    /// store.add(Packet::new(1, "my message".to_string())).unwrap();
    /// store.del_group(&1).unwrap();
    /// 
    /// assert!(store.get(GetOptions::default()).unwrap().is_none());
    /// 
    /// ```
    pub fn del_group(&mut self, priority: &u32) -> Result<(), Error> {
        if let Some(group) = self.groups_map.remove(priority) {
            for (uuid, _msg_byte_size) in group.msgs_map.iter() {
                self.id_to_group_map.remove(uuid);
                self.inc_msgs_deleted();
            }
            self.byte_size -= group.byte_size;            
        }        
        Ok(())
    }

    /// Gets a message from the store, either the next in line, the next in a specified priority group, or a specific message
    /// specified by the uuid option.
    /// 
    /// If the uuid option is present, it will search for that uuid only. If the priority option is present, it will retrieve the next
    /// message in line for that priority only. If neither options are present, the store will retrieve the next message in line store wide. 
    /// If no message is found, None is returned.
    /// 
    /// # Errors
    /// This method will return an error if the database encounters an error or if the store realizes that the state is out of sync.
    /// 
    /// # Example
    /// ```
    /// use msg_store::{ Packet, open, GetOptions };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(Packet::new(1, "my message".to_string())).unwrap();
    /// let mut get_options = GetOptions::default().uuid(uuid);
    /// let my_message = store.get(GetOptions::default().uuid(uuid)).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(GetOptions::default().priority(1)).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(GetOptions::default()).unwrap();
    /// assert!(my_message.is_some());
    /// 
    /// ```
    pub fn get(&mut self, uuid: Option<Uuid>, priority: Option<u32>, reverse: bool) -> Result<Option<Uuid>, Error> {

        if let Some(uuid) = uuid {

            match self.id_to_group_map.contains_key(&uuid) {
                true => Ok(Some(uuid)),
                false => Ok(None)
            }

        } else if let Some(priority) = priority {

            let group = match self.groups_map.get(&priority) {
                Some(group) => group,
                None => { return Ok(None) }
            };

            let uuid_option = match !reverse {
                true => group.msgs_map.keys().next(),
                false => group.msgs_map.keys().rev().next()
            };

            match uuid_option {
                Some(uuid) => Ok(Some(uuid.clone())),
                None => { return Ok(None) }
            }
            

        } else {

            let next_group_option = match !reverse {
                true => self.groups_map.values().rev().next(),
                false => self.groups_map.values().next()
            };

            let group = match next_group_option {
                Some(group) => group,
                None => { return Ok(None) }
            };

            let next_uuid_option = match !reverse {
                true => group.msgs_map.keys().next(),
                false => group.msgs_map.keys().rev().next()
            };

            match next_uuid_option {
                Some(uuid) => Ok(Some(uuid.clone())),
                None => Err(Error::SyncError)
            }

        }
    }

    /// Get x number of message metadata within a given range and/or priority. This can be useful in a larger application 
    /// context where more than one message retrieval may be required, like in a multithreaded app.
    /// 
    /// The range argument is a tulple consisting of two members. The first member is the starting index, and the second is the last index. 
    /// As always, indexes start with zero. If the priority argument is passed a integer the function will only return a vec containing metadata from that priority.
    /// 
    /// # Example
    /// 
    /// let mut store = open();
    /// let uuid1 = store.add(Packet::new(1, "my message".to_string())).unwrap();
    /// let uuid2 = store.add(Packet::new(1, "my second message".to_string())).unwrap();
    /// let uuid3 = store.add(Packet::new(1, "my thrid message".to_string())).unwrap();
    /// 
    /// let set = store.get_metadata((0,2), Some(1));
    /// assert_eq!(uuid1, set[0].uuid);
    /// assert_eq!(uuid2, set[1].uuid);
    /// assert_eq!(uuid3, set[2].uuid);
    /// 
    pub fn get_metadata(&mut self, range: (u32, u32), priority: Option<u32>) -> Vec<PacketMetaData> {
        let mut uuids = vec![];
        let mut iter_count: u32 = 0;
        let (start, end) = range;
        let mut primer_iter = 0;
        if let Some(priority) = priority {
            if let Some(group) = self.groups_map.get(&priority) {                
                for (uuid, msg_byte_size) in group.msgs_map.iter() {
                    if primer_iter < start {
                        primer_iter += 1;
                        continue;
                    }
                    uuids.push(PacketMetaData{
                        uuid: uuid.clone(),
                        priority: priority.clone(),
                        byte_size: msg_byte_size.clone()
                    });
                    if iter_count == end {
                        break;
                    }
                    iter_count += 1;
                }
            }
        } else {
            'group: for (priority, group) in self.groups_map.iter() {
                'msg: for (uuid, msg_byte_size) in group.msgs_map.iter() {
                    if primer_iter < start {
                        primer_iter += 1;
                        continue 'msg;
                    }
                    uuids.push(PacketMetaData{
                        uuid: uuid.clone(),
                        priority: priority.clone(),
                        byte_size: msg_byte_size.clone()
                    });
                    if iter_count == end {
                        break 'group;
                    }
                    iter_count += 1;
                }
            }
        }
        uuids
    }

    /// Updates the defaults for a priority group
    /// 
    /// The method takes a GroupDefaults struct which contains a member: max_byte_size.
    /// The max_byte_size member type is Option<u32>. This method will auto prune the group
    /// if the group's current bytesize is greater than the new max bytesize default.
    /// 
    /// # Errors
    /// The method will return an error if the database encounters an error
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
    /// store.add(Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(Packet::new(1, "bar".to_string())).unwrap();
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(3) });
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_group_defaults(&mut self, priority: u32, defaults: &GroupDefaults) -> Result<(u32, Vec<Uuid>), Error> {
        let mut bytes_removed = 0;
        let mut msgs_removed = vec![];
        self.group_defaults.insert(priority, defaults.clone());
        if let Some(mut group) = self.groups_map.remove(&priority) {
            group.update_from_config(defaults.clone());
            let (bytes_removed_from_group, mut msgs_removed_from_group) = self.prune_group(&mut group, 0, PruneBy::Group)?;
            bytes_removed += bytes_removed_from_group;
            msgs_removed.append(&mut msgs_removed_from_group);
            self.groups_map.insert(priority, group);
        }
        Ok((bytes_removed, msgs_removed))
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
    /// store.add(Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(Packet::new(1, "bar".to_string())).unwrap();
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
    pub fn delete_group_defaults(&mut self, priority: u32) {
        self.group_defaults.remove(&priority);
        if let Some(group) = self.groups_map.get_mut(&priority) {
            group.max_byte_size = None;
        }
    }

    /// Updates the defaults for the store
    /// 
    /// The method takes a StoreDefaults struct which contains a member: max_byte_size.
    /// The max_byte_size member type is Option<u32>. This method will auto prune the store
    /// if the store's current bytesize is greater than the new max bytesize default.
    /// 
    /// # Errors
    /// The method will return an error if the database encounters an error
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
    /// store.add(Packet::new(1, "foo".to_string())).unwrap();
    /// store.add(Packet::new(1, "bar".to_string())).unwrap();
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(3) }).unwrap();
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_store_defaults(&mut self, defaults: &StoreDefaults) -> Result<(u32, Vec<u32>, Vec<Uuid>), Error> {
        self.max_byte_size = defaults.max_byte_size;
        self.prune_store(None, u32::MAX, 0)
    }

}
