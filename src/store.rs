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
    pub groups_map: BTreeMap<GroupId, Group>
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
            groups_map: BTreeMap::new()
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
        self.db.del(&uuid)
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
                'groups: for (priority, group) in self.groups_map.iter_mut() {
                    if &packet.priority < priority {
                        break 'groups;
                    }
                    if !Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                        break 'groups;
                    }
                    let mut removed_msgs = RemovedMsgs::new(*priority);
                    'messages: for uuid in group.msgs_map.keys() {
                        if !Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                            break 'messages;
                        }
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
                if Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                    let mut removed_msgs = RemovedMsgs::new(packet.priority);
                    for uuid in group.msgs_map.keys() {
                        if !Self::msg_excedes_max_byte_size(&self.byte_size, &store_max_byte_size, &msg_byte_size) {
                            break;
                        }
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
                None => match self.id_to_group_map.keys().next() {
                    Some(uuid) => match self.db.get(&uuid) {
                        Some(msg) => Some(StoredPacket {
                            uuid: uuid.clone(),
                            msg
                        }),
                        None => None
                    },
                    None => None
                }
            }
        }
    }
}

// Store Actions Finish

// #[cfg(test)]
// mod tests {

//     use crate::store::{
//         delete, generate_store, get_next,
//         insert, GroupDefaults, Msg, MsgId, MsgByteSize
//     };

//     #[test]
//     fn should_insert_two_msgs_and_return_the_oldest_highest_msg() {
//         let mut store = generate_store();
//         let mut msg = Msg {
//             priority: 1,
//             byte_size: "0123456789".len() as MsgByteSize,
//         };
//         // insert one document
//         let msg_1 = insert(&mut store, &msg).unwrap();
//         assert_eq!(store.byte_size, 10);
//         assert_eq!(store.groups_map.len(), 1);
//         let group = store
//             .groups_map
//             .get(&msg.priority)
//             .expect("Collection not found");
//         assert_eq!(group.byte_size, 10);
//         assert_eq!(group.msgs_map.len(), 1);
//         // insert second document to same group
//         insert(&mut store, &msg).unwrap();
//         // get_next should get the oldest of the two
//         let returned_id = get_next(&mut store).unwrap().unwrap();
//         assert_eq!(msg_1.id, returned_id);
//         // insert a lower priority msg and then get_next should still return the older-higher priority msg
//         msg.priority = 0;
//         insert(&mut store, &msg).unwrap();
//         let returned_id = get_next(&mut store).unwrap().unwrap();
//         assert_eq!(msg_1.id, returned_id);
//     }

//     #[test]
//     fn should_reject_messages_that_exceed_max_sizes() {
//         let mut store = generate_store();
//         store.max_byte_size = Some(99);
//         let mut msg = Msg {
//             priority: 1,
//             byte_size: 100,
//         };
//         assert_eq!(true, insert(&mut store, &msg).is_err());
//         store.group_defaults.insert(
//             1,
//             GroupDefaults {
//                 max_byte_size: Some(99),
//             },
//         );
//         assert_eq!(true, insert(&mut store, &msg).is_err());
//         msg.byte_size = 99;
//         assert_eq!(true, insert(&mut store, &msg).is_ok());
//         msg.priority = 0;
//         assert_eq!(true, insert(&mut store, &msg).is_err());
//         {
//             let group = store.groups_map.get_mut(&1).unwrap();
//             group.max_byte_size = Some(9);
//         }
//         msg.priority = 1;
//         msg.byte_size = 10;
//         assert_eq!(true, insert(&mut store, &msg).is_err());
//     }

//     #[test]
//     fn should_delete_lower_priority_msgs() {
//         let mut store = generate_store();
//         store.max_byte_size = Some(10);
//         let mut msg = Msg {
//             priority: 0,
//             byte_size: 10,
//         };
//         let result_1 = insert(&mut store, &msg).unwrap();
//         let result_2 = insert(&mut store, &msg).unwrap();
//         {
//             let group = store.groups_map.get(&0).unwrap();
//             assert_eq!(result_1.id, result_2.ids_removed[0]);
//             assert_eq!(10, store.byte_size);
//             assert_eq!(10, group.byte_size);
//             assert_eq!(1, group.msgs_map.len());
//         }
//         msg.priority = 1;
//         let result_3 = insert(&mut store, &msg).unwrap();
//         let group_0 = store.groups_map.get(&0);
//         let group_1 = store.groups_map.get(&1).unwrap();
//         assert_eq!(result_2.id, result_3.ids_removed[0]);
//         assert_eq!(10, store.byte_size);
//         assert_eq!(10, group_1.byte_size);
//         assert_eq!(1, group_1.msgs_map.len());
//         assert_eq!(true, group_0.is_none());
//         assert_eq!(1, store.groups_map.len());
//     }

//     #[test]
//     fn should_removed_msgs_from_store() {
//         let mut store = generate_store();
//         let msg = Msg {
//             priority: 0,
//             byte_size: 10,
//         };
//         let insert_1 = insert(&mut store, &msg).unwrap();
//         let insert_2 = insert(&mut store, &msg).unwrap();
//         assert_eq!(true, delete(&mut store, &insert_1.id).is_ok());
//         assert_eq!(true, delete(&mut store, &insert_2.id).is_ok());
//         assert_eq!(0, store.byte_size);
//         assert_eq!(0, store.groups_map.len());
//     }

//     #[test]
//     fn it_should_removed_bytes_from_working_group() {
//         let mut store = generate_store();
//         store.group_defaults.insert(
//             0,
//             GroupDefaults {
//                 max_byte_size: Some(10),
//             },
//         );
//         let mut ids_inserted: Vec<MsgId> = vec![];
//         for _ in 1..=5 {
//             let result = insert(
//                 &mut store,
//                 &Msg {
//                     priority: 0,
//                     byte_size: 2,
//                 },
//             )
//             .unwrap();
//             ids_inserted.push(result.id);
//         }
//         let result_1 = insert(
//             &mut store,
//             &Msg {
//                 priority: 0,
//                 byte_size: 4,
//             },
//         )
//         .unwrap();
//         assert_eq!(ids_inserted[0], result_1.ids_removed[0]);
//         assert_eq!(ids_inserted[1], result_1.ids_removed[1]);
//         store.group_defaults.insert(
//             1,
//             GroupDefaults {
//                 max_byte_size: Some(10),
//             },
//         );
//         let result_2 = insert(
//             &mut store,
//             &Msg {
//                 priority: 1,
//                 byte_size: 5,
//             },
//         )
//         .unwrap();
//         let result_3 = insert(
//             &mut store,
//             &Msg {
//                 priority: 1,
//                 byte_size: 6,
//             },
//         )
//         .unwrap();
//         assert_eq!(result_2.id, result_3.ids_removed[0]);
//     }

//     #[test]
//     fn it_should_not_remove_group_but_msgs_from_group() {
//         let mut store = generate_store();
//         store.max_byte_size = Some(10);
//         let _result_1 = insert(
//             &mut store,
//             &Msg {
//                 priority: 1,
//                 byte_size: 5,
//             },
//         )
//         .unwrap();
//         let _result_2 = insert(
//             &mut store,
//             &Msg {
//                 priority: 1,
//                 byte_size: 5,
//             },
//         )
//         .unwrap();
//         let _result_3 = insert(
//             &mut store,
//             &Msg {
//                 priority: 2,
//                 byte_size: 5,
//             },
//         )
//         .unwrap();
//         let group_1 = store.groups_map.get(&1).unwrap();
//         let group_2 = store.groups_map.get(&2).unwrap();
//         assert_eq!(1, group_1.msgs_map.len());
//         assert_eq!(5, group_1.byte_size);
//         assert_eq!(10, store.byte_size);
//         assert_eq!(5, group_2.byte_size);
//         assert_eq!(1, group_2.msgs_map.len());
//     }

//     #[test]
//     fn it_should_return_ok_none_when_no_group_is_present() {
//         let mut store = generate_store();
//         assert_eq!(None, get_next(&mut store).unwrap());
//     }

// }
