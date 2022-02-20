pub mod get;
pub mod set;
pub mod rm;

#[cfg(test)]
pub mod tests {
    
    use bytes::Bytes;
    use crate::{stats::Stats, fake_payload};
    use crate::config::StoreConfig;
    use crate::file_storage::FileStorage;
    use crate::group_defaults::set::handle as set_handle;
    use crate::group_defaults::get::handle as get_handle;
    use crate::group_defaults::rm::handle as rm_handle;
    use crate::msg::tests::FakePayload;
    use crate::msg::add::handle as add_handle;
    use futures::executor::block_on;
    use msg_store::Store;
    use msg_store_database_plugin::Db;
    use msg_store_database_in_memory_plugin::MemDb;
    use std::fs::read_to_string;
    use std::sync::Mutex;
    use tempdir::TempDir;

    #[test]
    pub fn should_put_defaults_in_store() {

        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(MemDb::new()));
        let stats_mx = Mutex::new(Stats::new());
        let config_mx = Mutex::new(StoreConfig::new());
        let config_dir = TempDir::new("should_put_defaults_in_store-config-path").unwrap();
        let config_path = {
            let mut config_path = config_dir.path().to_path_buf();
            config_path.push("config.json");
            config_path
        };
        let file_storage_path = TempDir::new("should_put_defaults_in_store-file-storage").unwrap();
        let file_storage_op = Some(Mutex::new(FileStorage::new(file_storage_path.path()).unwrap()));

        // set group config
        block_on(set_handle(
            &store_mx,
            &database_mx,
            &file_storage_op,
            &stats_mx,
            &config_mx,
            &Some(config_path.to_path_buf()),
            1,
            Some(10)
        )).unwrap();

        {
            let store = store_mx.lock().unwrap();
            assert_eq!(store.group_defaults.get(&1).unwrap().max_byte_size.unwrap(), 10);
        }

        // insert msg
        let fake_payload = fake_payload!("priority=1?foo bar");

        block_on(add_handle(
            &store_mx,
            &file_storage_op,
            &stats_mx,
            &database_mx,
            fake_payload
        )).unwrap();

        // config path should have updated
        {
            assert!(config_path.as_path().exists());
            let config_json: StoreConfig = serde_json::from_str(&read_to_string(&config_path.as_path()).unwrap()).unwrap();
            assert_eq!(config_json.groups.unwrap()[0].max_byte_size.unwrap(), 10);
        }

        // new group should have defaults
        {
            let store = store_mx.lock().unwrap();
            assert_eq!(store.groups_map.get(&1).unwrap().max_byte_size.unwrap(), 10);
        }

        // get defaults
        let defaults = block_on(get_handle(
            &store_mx,
            Some(1)
        )).unwrap()[0];
        assert_eq!(defaults.max_byte_size.unwrap(), 10);

        // should prune msg in priority group
        block_on(set_handle(
            &store_mx,
            &database_mx,
            &file_storage_op,
            &stats_mx,
            &config_mx,
            &Some(config_path.to_path_buf()),
            1,
            Some(3)
        )).unwrap();

        {
            let store = store_mx.lock().unwrap();
            assert_eq!(store.group_defaults.get(&1).unwrap().max_byte_size.unwrap(), 3);
            assert_eq!(store.byte_size, 0);
            let mut database = database_mx.lock().unwrap();
            assert_eq!(database.fetch().unwrap().len(), 0);
            let stats = stats_mx.lock().unwrap();
            assert_eq!(stats.pruned, 1);
            let config_json: StoreConfig = serde_json::from_str(&read_to_string(&config_path.as_path()).unwrap()).unwrap();
            assert_eq!(config_json.groups.unwrap()[0].max_byte_size.unwrap(), 3);
        }

        block_on(rm_handle(
            &store_mx,
            &config_mx,
            &Some(config_path.to_path_buf()),
            1
        )).unwrap();
        {
            let store = store_mx.lock().unwrap();
            assert_eq!(store.group_defaults.len(), 0);
            let config_json: StoreConfig = serde_json::from_str(&read_to_string(&config_path.as_path()).unwrap()).unwrap();
            assert_eq!(config_json.groups.unwrap().len(), 0);
        }

    }

}