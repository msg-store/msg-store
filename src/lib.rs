
pub mod mem;
pub mod store;
pub mod uuid;

pub use crate::{
    store::{ GroupDefaults, Package, Packet, PacketMetaData, Store, StoreDefaults, StoredPacket},
    mem::{ MemStore, open },
    uuid::Uuid
};

/// This trait is used to create a database plugin for a store
pub trait Keeper {
    fn add(&mut self, package: &Package);
    fn get(&mut self, uuid: &Uuid) -> Option<String>;
    fn del(&mut self, uuid: &Uuid);
    fn fetch(&mut self) -> Vec<PacketMetaData>;
}

pub trait Storage<Db: Keeper> {

    fn open(db: Db) -> Store<Db>;

    /// A method for reseting the inserted messages count
    fn clear_msgs_inserted_count(&mut self);

    /// A method for reseting the burned messages count
    fn clear_msgs_burned_count(&mut self);

    /// A method for reseting the deleted messages count
    fn clear_msgs_deleted_count(&mut self);
   
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
    /// use msg_store::{ Packet, open, Storage };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");
    /// 
    /// ```
    /// 
    fn add(&mut self, packet: &Packet) -> Result<Uuid, String>;
    
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
    /// use msg_store::{ Packet, open, Storage };
    /// 
    /// let mut store = open();
    /// let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");
    /// store.del(&uuid).expect("Could not remove msg");
    /// 
    /// ```
    fn del(&mut self, uuid: &Uuid) -> Result<(), String>;

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
    /// use msg_store::{ Packet, open, Storage };
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
    fn get(&mut self, uuid: Option<Uuid>, priority: Option<i32>) -> Option<StoredPacket>;

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
    ///     open,
    ///     Storage
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
    fn update_group_defaults(&mut self, priority: i32, defaults: &GroupDefaults);

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
    ///     open,
    ///     Storage
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
    fn update_store_defaults(&mut self, defaults: &StoreDefaults);

}