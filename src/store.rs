use crate::{
    Keeper,
    uuid::{
        UuidManager,
        Uuid
    }
};

use std::collections::BTreeMap;

pub type MsgId = Uuid;
pub type GroupId = i32;
pub type MsgByteSize = i32;
type IdToGroup = BTreeMap<MsgId, GroupId>;

pub struct StoreDefaults {
    pub max_byte_size: Option<MsgByteSize>
}

#[derive(Debug, Clone, Copy)]
pub struct GroupDefaults {
    pub max_byte_size: Option<MsgByteSize>,
}

pub struct Group {
    pub max_byte_size: Option<MsgByteSize>,
    pub byte_size: MsgByteSize,
    pub msgs_map: BTreeMap<MsgId, MsgByteSize>,
}
impl Group {
    pub fn new(max_byte_size: Option<MsgByteSize>) -> Group {
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
    priority: GroupId,
    msgs: Vec<Uuid>
}
impl RemovedMsgs {
    pub fn new(priority: GroupId) -> RemovedMsgs {
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
    pub priority: GroupId,
    pub msg: String,
    pub byte_size: MsgByteSize
}

#[derive(Debug)]
pub struct StoredPacket {
    pub uuid: Uuid,
    pub msg: String
}

pub struct Packet {
    priority: GroupId,
    msg: String
}
impl Packet {
    pub fn new(priority: GroupId, msg: String) -> Packet {
        Packet { priority, msg }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PacketMetaData {
    pub uuid: Uuid,
    pub priority: GroupId,
    pub byte_size: MsgByteSize
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
    pub max_byte_size: Option<MsgByteSize>,
    pub byte_size: MsgByteSize,
    pub group_defaults: BTreeMap<GroupId, GroupDefaults>,
    pub uuid_manager: UuidManager,
    pub db: Db,
    pub id_to_group_map: IdToGroup,
    pub groups_map: BTreeMap<GroupId, Group>,
    pub msgs_inserted: i32,
    pub msgs_deleted: i32,
    pub msgs_burned: i32
}

impl<Db: Keeper> Store<Db> {

    pub fn open(db: Db) -> Store<Db> {        
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
        let mut data = store.db.fetch();
        data.sort();
        data.iter().for_each(|data| {
            store.insert_data(&data).expect("Could not insert data");
        });
        store
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

    fn msg_excedes_max_byte_size(byte_size: &MsgByteSize, max_byte_size: &MsgByteSize, msg_byte_size: &MsgByteSize) -> bool {
        &(byte_size + msg_byte_size) > max_byte_size
    }

    fn remove_msg(&mut self, uuid: &Uuid, group: &mut Group) {
        let byte_size = group.msgs_map.remove(&uuid).expect("Could not get msg byte size");
        self.id_to_group_map.remove(&uuid);
        self.byte_size -= byte_size;
        group.byte_size -= byte_size;
        self.db.del(&uuid);
        self.inc_msgs_burned_count();
    }
    
    fn get_group(&mut self, priority: GroupId) -> Group {
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

    fn check_msg_size_agains_store(&self, msg_byte_size: MsgByteSize) -> Result<(), String> {
        if let Some(store_max_byte_size) = self.max_byte_size {
            if msg_byte_size > store_max_byte_size {
                return Err("message excedes store max byte size".to_string())
            }
        }
        Ok(())
    }

    fn check_msg_size_against_group(&mut self, group: Group, msg_priority: GroupId, msg_byte_size: MsgByteSize) -> Result<Group, String> {
        // check if the msg is too large for the target group
        if let Some(group_max_byte_size) = &group.max_byte_size {
            if &msg_byte_size > group_max_byte_size {
                self.groups_map.insert(msg_priority, group);
                return Err("message excedes group max byte size.".to_string());
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
                return Err("message lacks priority.".to_string());
            }
        }
        Ok(group)
    }

    fn prune_group(&mut self, group: &mut Group, priority: GroupId, msg_byte_size: MsgByteSize) {
        if let Some(group_max_byte_size) = &group.max_byte_size {
            if Self::msg_excedes_max_byte_size(&group.byte_size, group_max_byte_size, &msg_byte_size) {
                // prune group
                let mut removed_msgs = RemovedMsgs::new(priority);
                let mut bytes_removed = 0;
                for (uuid, group_msg_byte_size) in group.msgs_map.iter() {
                    if !Self::msg_excedes_max_byte_size(&(group.byte_size - bytes_removed), group_max_byte_size, &msg_byte_size) {
                        break;
                    }
                    bytes_removed += group_msg_byte_size;
                    removed_msgs.add(uuid.clone());
                }
                for uuid in removed_msgs.msgs.iter() {
                    self.remove_msg(&uuid, group);
                }
            }
        }
    }

    fn prune_store(&mut self, group: &mut Group, msg_priority: GroupId, msg_byte_size: MsgByteSize) {
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
                    'messages: for (uuid, msg_byte_size) in group.msgs_map.iter() {
                        if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                            break 'messages;
                        }
                        bytes_removed += msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }
                    if group.byte_size == 0 {
                        groups_removed.push(*priority);
                    }
                    all_removed_msgs.push(removed_msgs);
                }
                // get groups of msgs that where removed
                for group_data in all_removed_msgs {
                    let mut group = self.groups_map.remove(&group_data.priority).expect("Could not find mutable group");
                    for uuid in group_data.msgs {
                        self.remove_msg(&uuid, &mut group);
                    }
                    self.groups_map.insert(group_data.priority, group);
                }
                for priority in groups_removed {
                    self.groups_map.remove(&priority);
                }

                // prune group again
                if Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                    let mut removed_msgs = RemovedMsgs::new(msg_priority);
                    let mut bytes_removed = 0;
                    for uuid in group.msgs_map.keys() {
                        if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                            break;
                        }
                        bytes_removed = msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }
                    for uuid in removed_msgs.msgs {
                        self.remove_msg(&uuid, group);
                    }
                }
            }            
        }
    }

    fn insert_msg(&mut self, mut group: Group, uuid: Uuid, priority: GroupId, msg_byte_size: MsgByteSize) {
        self.byte_size += msg_byte_size;                                          // increase store byte size
        self.id_to_group_map.insert(uuid.clone(), priority);            // insert the uuid into the uuid->priority map
        group.byte_size += msg_byte_size;                                         // increase the group byte size
        group.msgs_map.insert(uuid.clone(), msg_byte_size);             // insert the uuid into the uuid->byte size map
        self.groups_map.insert(priority, group);
    }

    fn insert_data(&mut self, data: &PacketMetaData) -> Result<(), String> {

        // check if the msg is too large for the store
        self.check_msg_size_agains_store(data.byte_size)?;

        // check if the target group exists
        // create if id does not
        let group = self.get_group(data.priority);

        // check if the msg is too large for the target group
        let mut group = self.check_msg_size_against_group(group, data.priority, data.byte_size)?;

        // prune group if needed
        self.prune_group(&mut group, data.priority, data.byte_size);

        // prune store
        self.prune_store(&mut group, data.priority, data.byte_size);

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
    /// * there is an error while reading or writing to disk.
    /// 
    /// The error wiil be returned as a string.
    /// 
    /// # Examples
    /// ```
    /// use msg_store::{
    ///     store::Packet,
    ///     database::mem::open
    /// };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");
    /// 
    /// ```
    /// 
    pub fn add(&mut self, packet: &Packet) -> Result<Uuid, String> {

        let msg_byte_size = packet.msg.len() as MsgByteSize;

        // check if the msg is too large for the store
        self.check_msg_size_agains_store(msg_byte_size)?;

        // check if the target group exists
        // create if id does not
        let group = self.get_group(packet.priority);

        // check if the msg is too large for the target group
        let mut group = self.check_msg_size_against_group(group, packet.priority, msg_byte_size)?;

        // prune group if needed
        self.prune_group(&mut group, packet.priority, msg_byte_size);

        // prune store
        self.prune_store(&mut group, packet.priority, msg_byte_size);

        // insert msg
        let uuid = self.uuid_manager.next();                                 // get uuid
        self.insert_msg(group, uuid, packet.priority, msg_byte_size);
        
        let package = Package {
            uuid,
            priority: packet.priority,
            msg: packet.msg.clone(),
            byte_size: msg_byte_size
        };
        self.db.add(&package);
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
    /// use msg_store::{
    ///     store::Packet,
    ///     database::mem::open
    /// };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");
    /// store.del(&uuid).expect("Could not remove msg");
    /// 
    /// ```
    pub fn del(&mut self, uuid: &Uuid) -> Result<(), String> {
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
        self.db.del(&uuid);
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
    /// use msg_store::{
    ///     store::Packet,
    ///     database::mem::open
    /// };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");
    /// let my_message = store.get(Some(uuid), None);
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(None, Some(1));
    /// assert!(my_message.is_some());
    /// 
    /// let my_message = store.get(None, None);
    /// assert!(my_message.is_some());
    /// 
    /// ```
    pub fn get(&mut self, uuid: Option<Uuid>, priority: Option<GroupId>) -> Option<StoredPacket> {
        match uuid {
            Some(uuid) => match self.db.get(&uuid) {
                Some(msg) => Some(StoredPacket {
                    uuid,
                    msg
                }),
                None => None
            },
            None => match priority {
                Some(priority) => match self.groups_map.get(&priority) {
                    Some(group) => match group.msgs_map.keys().next() {
                        Some(uuid) => match self.db.get(&uuid) {
                            Some(msg) => Some(StoredPacket {
                                uuid: uuid.clone(),
                                msg
                            }),
                            None => None
                        },
                        None => None
                    },
                    None => None
                },                
                None => match self.groups_map.values().rev().next() {
                    Some(group) => {
                        let uuid = group.msgs_map.keys().next().expect("Could not find message");
                        let msg = self.db.get(&uuid).expect("Could not find message");
                        Some(StoredPacket {
                            uuid: uuid.clone(),
                            msg
                        })
                    },
                    None => None
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
    ///     store::{GroupDefaults, Packet},
    ///     database::mem::open
    /// };
    /// 
    /// let mut store = open();
    /// store.add(&Packet::new(1, "foo".to_string())).expect("Could not add msg");
    /// store.add(&Packet::new(1, "bar".to_string())).expect("Could not add msg");
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_group_defaults(1, &GroupDefaults{ max_byte_size: Some(3) });
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_group_defaults(&mut self, priority: GroupId, defaults: &GroupDefaults) {
        self.group_defaults.insert(priority, defaults.clone());
        if let Some(mut group) = self.groups_map.remove(&priority) {
            group.update_from_config(defaults.clone());
            self.prune_group(&mut group, priority, 0);
            self.groups_map.insert(priority, group);
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
    ///     store::{Packet, StoreDefaults},
    ///     database::mem::open
    /// };
    /// 
    /// let mut store = open();
    /// store.add(&Packet::new(1, "foo".to_string())).expect("Could not add msg");
    /// store.add(&Packet::new(1, "bar".to_string())).expect("Could not add msg");
    /// assert_eq!(6, store.byte_size); // The store should contain 6 bytes of data, 3 for each message.
    /// 
    /// store.update_store_defaults(&StoreDefaults{ max_byte_size: Some(3) });
    /// 
    /// // The store should have removed 3 bytes in order to abide by the new requirement
    /// assert_eq!(3, store.byte_size); 
    /// 
    /// ```
    pub fn update_store_defaults(&mut self, defaults: &StoreDefaults) {
        self.max_byte_size = defaults.max_byte_size;
        if let Some(store_max_byte_size) = self.max_byte_size.clone() {
            if Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &0) {
                let mut groups_removed = vec![];
                let mut all_removed_msgs = vec![];
                let mut bytes_removed = 0;
                'groups: for (priority, group) in self.groups_map.iter_mut() {
                    if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &0) {
                        break 'groups;
                    }
                    let mut removed_msgs = RemovedMsgs::new(*priority);
                    'messages: for (uuid, msg_byte_size) in group.msgs_map.iter() {
                        if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &0) {
                            break 'messages;
                        }
                        bytes_removed += msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }
                    if group.byte_size == 0 {
                        groups_removed.push(*priority);
                    }
                    all_removed_msgs.push(removed_msgs);
                }
                // get groups of msgs that where removed
                for group_data in all_removed_msgs {
                    let mut group = self.groups_map.remove(&group_data.priority).expect("Could not find mutable group");
                    for uuid in group_data.msgs {
                        self.remove_msg(&uuid, &mut group);
                    }
                    self.groups_map.insert(group_data.priority, group);
                }
                for priority in groups_removed {
                    self.groups_map.remove(&priority);
                }
            }            
        }
    }

}
