
pub mod mem;
pub mod store;
pub mod uuid;

pub use crate::{
    store::{ Package, Packet, PacketMetaData, Store},
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