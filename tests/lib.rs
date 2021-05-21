
use msg_store_rs::{store::{Store}};
use msg_store_rs::config::{StoreConfig, CollectionConfig};
use std::time;
// use std::collections::HashMap;
use tempdir::TempDir;

#[test]
fn it_should_work() {

  let dir = TempDir::new("/home/joshua/tmp").unwrap();
  let priority_5 = CollectionConfig {
    priority: 5,
    limit: Some(10)
  };
  let config = StoreConfig {
    dir: dir.path().to_path_buf(),
    limit: None,
    collections: Some(vec![
      priority_5
    ])
  };
  let mut store = Store::new(config).unwrap();
 
  store.add(&1, &"0000000001".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000001", &String::from_utf8(packet.msg).unwrap());
  
  store.add(&1, &"0000000002".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000001", &String::from_utf8(packet.msg).unwrap());

  store.del(&packet.id).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000002", &String::from_utf8(packet.msg).unwrap());

  store.del(&packet.id).unwrap();
  let packet = store.get().unwrap();
  assert_eq!(None, packet);

  store.add(&5, &"0000000001".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000001", &String::from_utf8(packet.msg).unwrap());
  
  store.add(&5, &"0000000002".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000002", &String::from_utf8(packet.msg).unwrap());

  store.del(&packet.id).unwrap();
  let packet = store.get().unwrap();
  assert_eq!(None, packet);

  store.add(&2, &"0000000001".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000001", &String::from_utf8(packet.msg).unwrap());

  store.add(&1, &"0000000002".as_bytes().to_vec()).unwrap();
  let packet = store.get().unwrap().unwrap();
  assert_eq!("0000000002", &String::from_utf8(packet.msg).unwrap());

}

#[test]
fn bench () {
  let dir = TempDir::new("/home/joshua/tmp").unwrap();
  let priority_5 = CollectionConfig {
    priority: 5,
    limit: Some(10)
  };
  let config = StoreConfig {
    dir: dir.path().to_path_buf(),
    limit: None,
    collections: Some(vec![
      priority_5
    ])
  };
  let mut store = Store::new(config).unwrap();

  let message = "hello, world!".as_bytes().to_vec();
  let start_time = time::Instant::now();
  store.add(&1, &message).unwrap();
  let end_time = start_time.elapsed().as_micros();
  println!("{}", end_time);
}