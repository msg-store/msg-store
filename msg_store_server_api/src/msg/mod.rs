pub mod add;
pub mod get;
pub mod rm;

#[cfg(test)]
pub mod tests {
    use bytes::Bytes;
    use msg_store::{Store, StoreDefaults, GroupDefaults};
    use crate::file_storage::FileStorage;
    use crate::stats::Stats;
    use msg_store_database_plugin::Db;
    use msg_store_database_in_memory_plugin::MemDb;
    use futures::{Stream, StreamExt};
    use futures::executor::block_on;
    use std::sync::Mutex;
    use std::task::Poll;
    use super::add::{handle as add_handle, Chunky, AddErrorTy, MsgError};
    use super::get::{handle as get_handle, ReturnBody};
    use super::rm::handle as rm_handle;
    use tempdir::TempDir;

    pub struct FakePayload {
        pub msg: Bytes,
        pub done: bool
    }
    impl Stream for FakePayload {
        type Item = Result<Bytes, &'static str>;
        fn poll_next(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Option<Self::Item>> {
            if self.done {
                Poll::Ready(None)
            } else {
                self.done = true;
                Poll::Ready(Some(Ok(Bytes::copy_from_slice(&self.msg))))
            }
        }
    }
    impl Chunky for FakePayload {}

    #[macro_export]
    macro_rules! fake_payload {
        ($msg:expr) => {
            {
                let msg_str = $msg.to_string();
                let chunk = Bytes::copy_from_slice(&msg_str.as_bytes());
                FakePayload {
                    msg: chunk,
                    done: false
                }
            }
        };
    }

    async fn convert_return_body_msg_to_string(mut return_body: ReturnBody) -> String {
        let mut payload_str = String::new();
        while let Some(chunk_rst) = return_body.next().await {
            let chunk = chunk_rst.unwrap();
            payload_str.push_str(&String::from_utf8(chunk.to_vec()).unwrap())
        }
        payload_str
    }

    #[test]
    fn should_add_get_and_rm_msg() {
        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(MemDb::new()));
        let stats_mx = Mutex::new(Stats::new());
        let tmp_dir = TempDir::new("should_add_get_and_rm_msg").unwrap();
        let file_storage_op = Some(Mutex::new(FileStorage::new(tmp_dir.path()).unwrap()));

        let msg = "Hello, world";
        let msg_len = msg.len() as u64;
        let payload_str = format!("priority=1?{}", msg);
        let payload = fake_payload!(payload_str);

        // insert msg
        let uuid = block_on(add_handle(
            &store_mx, 
            &file_storage_op,
            &stats_mx, 
            &database_mx, 
            payload)).unwrap();
        
        // make insert assertions
        {
            let store = store_mx.lock().unwrap();                    // Lock the store
            let mut database = database_mx.lock().unwrap();     // Lock the database
            let stats = stats_mx.lock().unwrap();                    // Lock the stats
            assert_eq!(msg_len, store.byte_size);                                    // The number of bytes in the store should match the msg len
            let data = database.fetch().unwrap();                 // Get all data in the database
            assert_eq!(1, data.len());                                               // there should be 1 msg in the database
            assert_eq!(1, stats.inserted)                                            // the stats should reflect 1 msg insterted
        }

        // get a msg string
        let received_payload = block_on(get_handle(
            &store_mx, 
            &database_mx, 
            &file_storage_op, 
            Some(uuid.clone()), 
            None, 
            false)).unwrap().unwrap().b();
        
        // make get assertions
        {
            assert_eq!(format!("uuid={}?{}", uuid.to_string(), msg), received_payload);
        }

        let payload_str = format!("priority=1&saveToFile=true&bytesizeOverride={}?{}",msg_len, msg);
        let payload = fake_payload!(payload_str);
        
        // insert 'stream'
        let uuid_stream = block_on(add_handle(
            &store_mx, 
            &file_storage_op,
            &stats_mx, 
            &database_mx, 
            payload)).unwrap();
        
        // make insert 'file' assertions
        {
            let file_path = {
                let mut file_path = tmp_dir.path().to_path_buf();
                file_path.push(uuid_stream.to_string());
                file_path
            };
            assert!(file_path.exists())
        }

        // get a msg string
        let received_payload = block_on(get_handle(
            &store_mx, 
            &database_mx, 
            &file_storage_op, 
            Some(uuid_stream.clone()), 
            None, 
            false)).unwrap().unwrap().a();
        
        // make get assertions
        {
            assert_eq!(
                format!("uuid={}&bytesizeOverride={}&saveToFile=true?{}", uuid_stream.to_string(), msg.len(), msg), 
                block_on(convert_return_body_msg_to_string(received_payload)));
        }

        // remove msgs
        block_on(rm_handle(
            &store_mx, 
            &database_mx, 
            &file_storage_op, 
            &stats_mx, uuid.clone())).unwrap();

            block_on(rm_handle(
            &store_mx, 
            &database_mx, 
            &file_storage_op, 
            &stats_mx, uuid_stream.clone())).unwrap();
        
        // make rm assertions
        {
            let store = store_mx.lock().unwrap();                    // Lock the store
            let mut database = database_mx.lock().unwrap();     // Lock the database
            let stats = stats_mx.lock().unwrap();                    // Lock the stats
            assert_eq!(0, store.byte_size);                                          // The number of bytes in the store should match the msg len
            let data = database.fetch().unwrap();                 // Get all data in the database
            assert_eq!(0, data.len());                                               // there should be 1 msg in the database
            assert_eq!(2, stats.deleted);                                            // the stats should reflect 1 msg insterted
            let file_path = {
                let mut file_path = tmp_dir.path().to_path_buf();
                file_path.push(uuid_stream.to_string());
                file_path
            };
            assert!(!file_path.exists())                                             // The file should not exist
        }

        // reinitialize the store
        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(MemDb::new()));
        let stats_mx = Mutex::new(Stats::new());
        // update store
        {
            let mut store = store_mx.lock().unwrap();
            store.update_store_defaults(&StoreDefaults { max_byte_size: Some(3) }).unwrap();
        }

        let msg = "foo";
        let msg_len = msg.len() as u64;
        let payload_str = format!("priority=1?{}", msg);
        let payload = fake_payload!(payload_str);

        // insert 2 msgs
        block_on(add_handle(
            &store_mx, 
            &file_storage_op,
            &stats_mx, 
            &database_mx, 
            payload)).unwrap();
        
        let payload = fake_payload!(payload_str);
        block_on(add_handle(
            &store_mx, 
            &file_storage_op,
            &stats_mx, 
            &database_mx, 
            payload)).unwrap();

        // make pruned assertions
        {
            let store = store_mx.lock().unwrap();                    // Lock the store
            let mut database = database_mx.lock().unwrap();     // Lock the database
            let stats = stats_mx.lock().unwrap();                    // Lock the stats
            assert_eq!(msg_len, store.byte_size);                                    // The number of bytes in the store should match the msg len
            let data = database.fetch().unwrap();                 // Get all data in the database
            assert_eq!(1, data.len());                                               // there should be 1 msg in the database
            assert_eq!(1, stats.pruned)                                              // the stats should reflect 1 msg insterted
        }
        
    }

    #[test]
    fn should_reject_messages() {
        let store_mx = Mutex::new(Store::new(None).unwrap());
        let database_mx: Mutex<Box<dyn Db>> = Mutex::new(Box::new(MemDb::new()));
        let stats_mx = Mutex::new(Stats::new());
        let tmp_dir = TempDir::new("should_add_get_and_rm_msg").unwrap();
        let file_storage_op = Some(Mutex::new(FileStorage::new(tmp_dir.path()).unwrap()));

        {
            // should reject file storage not being configured
            let payload = fake_payload!("priority=1&saveToFile=true&bytesizeoverhead=1?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &None,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::FileStorageNotConfigured, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        {
            // should reject for invalid bytesizeOverride
            let payload = fake_payload!("priority=1&saveToFile=true&bytesizeOverride=true?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::InvalidBytesizeOverride, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        {
            // should reject for invalid priority
            let payload = fake_payload!("priority=mypriority?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::InvalidPriority, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        {
            // should reject for missing bytesizeOverride
            let payload = fake_payload!("priority=1&saveToFile=true?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MissingBytesizeOverride, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        // {
        //     // should reject for missing headers
        //     let payload = fake_payload!("my-msg");
        //     let add_err = block_on(add_handle(
        //         &store_mx, 
        //         &file_storage_op,
        //         &stats_mx, 
        //         &database_mx, 
        //         payload)).err().unwrap();
        //     if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
        //         assert_eq!(MsgError::MissingHeaders, msg_err)
        //     } else {
        //         panic!("Not a msg error");
        //     }
        // }

        {
            // should reject for missing priority
            let payload = fake_payload!("myparam=4?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MissingPriority, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        {
            // should reject msg for malformed headers
            let payload = fake_payload!("myheaders?my-msg");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MalformedHeaders, msg_err)
            } else {
                panic!("Not a msg error");
            }
        }

        {
            let store_mx = {
                let mut store = Store::new(None).unwrap();
                store.update_group_defaults(1, &GroupDefaults { max_byte_size: Some(3) }).unwrap();
                Mutex::new(store)
            };
            // should reject msg for exceeding the group max            
            let payload = fake_payload!("priority=1?for bar");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MsgExceedesGroupMax, msg_err)
            } else {
                panic!("Not a msg error");
            }            
        }

        {
            // should reject msg for exceeding the store max
            let store_mx = {
                let mut store = Store::new(None).unwrap();
                store.max_byte_size = Some(3);
                Mutex::new(store)
            };
            // should reject msg for exceeding the group max
            let payload = fake_payload!("priority=1?for bar");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MsgExceedesStoreMax, msg_err)
            } else {
                panic!("Not a msg error");
            }            
        }

        {
            // should reject msg for lacking priority
            let store_mx = {
                let mut store = Store::new(None).unwrap();
                store.max_byte_size = Some(3);
                Mutex::new(store)
            };
            // should reject msg for exceeding the group max
            let payload = fake_payload!("priority=2?foo");
            block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).unwrap();
            let payload = fake_payload!("priority=1?foo");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            if let AddErrorTy::MsgError(msg_err) = add_err.err_ty {
                assert_eq!(MsgError::MsgLacksPriority, msg_err)
            } else {
                panic!("Not a msg error");
            }            
        }

        {
            // should reject msg for exceeding the store max
            let store_mx = {
                let mut store = Store::new(None).unwrap();
                store.max_byte_size = Some(3);
                Mutex::new(store)
            };
            // should reject msg for exceeding the group max
            let payload = fake_payload!("priority=1?for bar");
            let add_err = block_on(add_handle(
                &store_mx, 
                &file_storage_op,
                &stats_mx, 
                &database_mx, 
                payload)).err().unwrap();
            assert!(add_err.to_string().contains("ADD_MSG_ERROR: (MSG_ERROR: MsgExceedesStoreMax). "))
        }

    }
}