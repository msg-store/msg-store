
pub mod errors;
pub mod mem;
pub mod store;
pub mod uuid;

pub use crate::{
    errors::DbError,
    store::{ Package, Packet, PacketMetaData, Store},
    mem::{ MemStore, open },
    uuid::Uuid
};

/// This trait is used to create a database plugin for a store
pub trait Keeper {
    fn add(&mut self, package: &Package) -> Result<(), DbError>;
    fn get(&mut self, uuid: &Uuid) -> Result<Option<String>, DbError>;
    fn del(&mut self, uuid: &Uuid) -> Result<(), DbError>;
    fn fetch(&mut self) -> Result<Vec<PacketMetaData>, DbError>;
}