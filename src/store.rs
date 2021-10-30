use crate::{
    Keeper,
    uuid::{
        UuidManager,
        Uuid
    }
};
#[cfg(feature = "level")]
use serde::{Deserialize, Serialize};
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

#[cfg_attr(feature = "level", derive(Serialize, Deserialize))]
pub struct PacketMetaData {
    pub uuid: Uuid,
    pub priority: GroupId,
    pub byte_size: MsgByteSize
}

pub struct Store<Db: Keeper> {
    pub max_byte_size: Option<MsgByteSize>,
    pub byte_size: MsgByteSize,
    pub group_defaults: BTreeMap<GroupId, GroupDefaults>,
    pub uuid_manager: UuidManager,
    pub db: Db,
    pub id_to_group_map: IdToGroup,
    pub groups_map: BTreeMap<GroupId, Group>,
    pub msgs_deleted: i32,
    pub msgs_burned: i32
}

impl<Db: Keeper> Store<Db> {

    pub fn new(db: Db) -> Store<Db> {
        Store {
            max_byte_size: None,
            byte_size: 0,
            group_defaults: BTreeMap::new(),
            uuid_manager: UuidManager::default(),
            db,
            id_to_group_map: BTreeMap::new(),
            groups_map: BTreeMap::new(),
            msgs_deleted: 0,
            msgs_burned: 0
        }
    }

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
    
    pub fn add(&mut self, packet: &Packet) -> Result<Uuid, String> {

        let msg_byte_size = packet.msg.len() as MsgByteSize;

        // check if the msg is too large for the store
        if let Some(store_max_byte_size) = self.max_byte_size {
            if msg_byte_size > store_max_byte_size {
                return Err("message excedes store max byte size".to_string())
            }
        }

        // check if the target group exists
        // create if id does not
        let mut group = match self.groups_map.remove(&packet.priority) {
            Some(group) => group,
            None => {
                let max_byte_size = match self.group_defaults.get(&packet.priority) {
                    Some(defaults) => defaults.max_byte_size.clone(),
                    None => None
                };
                Group::new(max_byte_size)
            }
        };

        // check if the msg is too large for the target group
        if let Some(group_max_byte_size) = &group.max_byte_size {
            if &msg_byte_size > group_max_byte_size {
                self.groups_map.insert(packet.priority, group);
                return Err("message excedes group max byte size.".to_string());
            }
        }

        // get the total byte count of all msgs that are higher priority
        // in order to know how free bytes are remaining for the new message
        let higher_priority_msg_total = {
            let mut total = 0;
            for (priority, group) in self.groups_map.iter().rev() {
                if &packet.priority > priority {
                    break;
                }
                total += group.byte_size;
            }
            total
        };

        // check if there is enough free space for the message
        if let Some(store_max_byte_size) = self.max_byte_size {
            if Self::msg_excedes_max_byte_size(&higher_priority_msg_total, &store_max_byte_size, &msg_byte_size) {
                return Err("message lacks priority.".to_string());
            }
        }

        // prune group if needed
        if let Some(group_max_byte_size) = &group.max_byte_size {
            if Self::msg_excedes_max_byte_size(&group.byte_size, group_max_byte_size, &msg_byte_size) {
                // prune group
                let mut removed_msgs = RemovedMsgs::new(packet.priority);
                for (uuid, msg_byte_size) in group.msgs_map.iter() {
                    if !Self::msg_excedes_max_byte_size(&group.byte_size, group_max_byte_size, &msg_byte_size) {
                        break;
                    }
                    removed_msgs.add(uuid.clone());
                }
                for uuid in removed_msgs.msgs.iter() {
                    self.remove_msg(&uuid, &mut group);
                }
            }
        }

        // prune store
        if let Some(store_max_byte_size) = self.max_byte_size.clone() {
            if Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                let mut groups_removed = vec![];
                let mut all_removed_msgs = vec![];
                let mut bytes_removed = 0;
                'groups: for (priority, group) in self.groups_map.iter_mut() {
                    if &packet.priority < priority {
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
                    let mut removed_msgs = RemovedMsgs::new(packet.priority);
                    let mut bytes_removed = 0;
                    for uuid in group.msgs_map.keys() {
                        if !Self::msg_excedes_max_byte_size(&(self.byte_size - bytes_removed), &store_max_byte_size, &msg_byte_size) {
                            break;
                        }
                        bytes_removed = msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }
                    for uuid in removed_msgs.msgs {
                        self.remove_msg(&uuid, &mut group);
                    }
                }
            }            
        }

        // insert msg
        let uuid = self.uuid_manager.next();                                 // get uuid
        self.byte_size += msg_byte_size;                                          // increase store byte size
        self.id_to_group_map.insert(uuid.clone(), packet.priority);     // insert the uuid into the uuid->priority map
        group.byte_size += msg_byte_size;                                         // increase the group byte size
        group.msgs_map.insert(uuid.clone(), msg_byte_size);             // insert the uuid into the uuid->byte size map
        self.groups_map.insert(packet.priority, group);

        let package = Package {
            uuid,
            priority: packet.priority,
            msg: packet.msg.clone(),
            byte_size: msg_byte_size
        };
        self.db.add(&package);

        Ok(uuid)
        
    }
    
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

    pub fn update_group_defaults(&mut self, priority: GroupId, defaults: &GroupDefaults) {
        self.group_defaults.insert(priority, defaults.clone());
        if let Some(mut group) = self.groups_map.remove(&priority) {
            group.update_from_config(defaults.clone());
            if let Some(max_byte_size) = group.max_byte_size {
                let mut removed_msgs = RemovedMsgs::new(priority);
                let mut bytes_removed = 0;
                if Self::msg_excedes_max_byte_size(&(group.byte_size - bytes_removed), &max_byte_size, &0) {                    
                    for (uuid, msg_byte_size) in group.msgs_map.iter() {
                        if !Self::msg_excedes_max_byte_size(&(group.byte_size - bytes_removed), &max_byte_size, &0) {
                            break;
                        }
                        println!("group byte size: {}, max byte size: {}, removing: {:#?}", &(group.byte_size - bytes_removed), max_byte_size, uuid);
                        bytes_removed += msg_byte_size;
                        removed_msgs.add(uuid.clone());
                    }                    
                }
                for uuid in removed_msgs.msgs {
                    self.remove_msg(&uuid, &mut group);
                }
            }
            self.groups_map.insert(priority, group);
        }
    }

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
