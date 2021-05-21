#[macro_use]
extern crate bencher;
use bencher::Bencher;
use msg_store_rs::{store::{Store}};
use msg_store_rs::config::{StoreConfig, CollectionConfig};
use tempdir::TempDir;

fn add(bench: &mut Bencher) {
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
  bench.iter(|| {
    store.add(&1, &message)
  })
}

benchmark_group!(benches, add);
benchmark_main!(benches);
