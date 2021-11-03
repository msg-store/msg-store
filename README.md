# msg-store
The message store is fast and lightweight and allows for messages to be forwarded on a priority basis where bandwidth and disk capacity may be limited.

## Getting started
In Cargo.toml
```toml
msg_store = "0.1.0"
```

In src/main.rs
```rust
    use msg_store::{ Packet, open };
    
    let mut store = open();
    let uuid = store.add(&Packet::new(1, "my message".to_string())).expect("Could not add msg");

    let my_message = store.get(None, None);
    // or
    let my_message = store.get(None, Some(uuid))
```

* Messages are forwarded on a highest priority then oldest status basis.
* Messages are burned/pruned on a lowest priority then oldest status basis.
* Messages are only burned once the store has reached the max byte size limit.
* The store as a whole contains a max byte size limit option, as does each individual priority
* group. For example, a developer can limit the size of the store to 1,000 bytes, while restricting
* priority group 1 to only 500 bytes, and leave higher priorities free with no restriction (except that of the store.)
* 
* The store keeps track of basic statistics such as counting the messages that have been inserted, deleted, or burned.
* Messages that have been deleted have been so on instructions of the developer using the del method.
* Messages that have been burned have been so automatically on insert or store/group defaults update once the
max byte size limit has been reached.
* The store can contain 2,147,483,647 priorities.

## Example
Four message are needed to be forwarded to a distant server.
The first message is placed in priority 1, the second in priority 2, the third message in priority 1, and the fourth in priority 2 again as shown in the rust example and table below.
```rust
    use msg_store::{
        Packet,
        open
    };
    
    let mut store = open();
    store.add(&Packet::new(1, "msg 1".to_string())).expect("Could not add msg");
    store.add(&Packet::new(2, "msg 2".to_string())).expect("Could not add msg");
    store.add(&Packet::new(1, "msg 3".to_string())).expect("Could not add msg");
    store.add(&Packet::new(2, "msg 4".to_string())).expect("Could not add msg");

    let msg_2 = store.get(None, None);
    
```

| priority 1 | priority 2 |
|:----------:|:----------:|
| msg 1      | msg 2      |
| msg 3      | msg 4      |

When get is called, msg 2 will be the first retrieved, because it is in the highest priority group and is also the oldest message in that group. The second msg retrieved would be msg 4.

While this may be the default, it is not strictly enforced. A developer could pass a priority to the get method to get the next message from that group, or one could also pass the uuid to the method to get the exact message desired.
```rust
    use msg_store::{
        Packet,
        open
    };
    
    let mut store = open();
    let msg_1_uuid = store.add(&Packet::new(1, "msg 1".to_string())).expect("Could not add msg");
    let msg_2_uuid = store.add(&Packet::new(2, "msg 2".to_string())).expect("Could not add msg");
    let msg_3_uuid = store.add(&Packet::new(1, "msg 3".to_string())).expect("Could not add msg");
    let msg_4_uuid = store.add(&Packet::new(2, "msg 4".to_string())).expect("Could not add msg");

    let msg_2 = store.get(None, None).expect("Could not get msg");
    let msg_1 = store.get(Some(1), None).expect("Could not get msg");
    let msg_3 = store.get(None, Some(msg_3_uuid)).expect("Could not get msg");
    
```

On the other hand if there is a max byte size limit set, the first message to be pruned would msg 1, because it is in the lowest priority group and also the oldest message in that group. The second message pruned would be msg 3.

## Database backends
The message store is designed to be database agnostic and could theoretically work with any database as a backend provided that a developer writes the glue code.

This library provides the Keeper trait which must be implemented for any backend plugin. 

Other plugins can be found on the [msg-store github page](https://github.com/msg-store)
