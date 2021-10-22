
pub mod database;
pub mod store;
pub mod uuid;

use crate::{
    uuid::Uuid,
    store::{ Package, PacketMetaData }
};

pub trait Keeper {
    fn add(&mut self, package: &Package);
    fn get(&mut self, uuid: &Uuid) -> Option<String>;
    fn del(&mut self, uuid: &Uuid);
    fn fetch(&mut self) -> Vec<PacketMetaData>;
}