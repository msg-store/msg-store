# msg-store-http-client
A js client for the msg-store http server

## Getting Started
```bash
npm i msg-store-http-client
```

```javascript
import msgStore from "msg-store-http-client"
const { 
    addMsg, getMsg, getNext, deleteMsg, 
    getGroup, deleteGroup,
    getStats, updateStats, deleteStats,
    getStore, updateStore,
    getGroupDefaults, setGroupDefaults, deleteGroupDefaults,
    getStream, addStream,
    exportMsgs
} = msgStore("http://127.0.0.1:8080")

const main = async () => {

    const uuid = await addMsg(1, "hello, world!").data.uuid

    const msg = await getMsg({ uuid }).data.msg // => "hello, world!"

}

main()

```

---
`addMsg(<number>, <string>)`  
* `priority` **number** the priority to group to place in the msg in.  
* `msg` **string** the msg to put in the store.  

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?** The data returned if statusCode is 200
        * `data.uuid` **string** The uuid of the msg inserted   

Add a message to the store
* Returns `400` if received a bad request.
* Returns `403` if attempting to save to file if the server file storage in not configured.
* Returns `409` if:   
    * The `msg` exceeds the store max byte size
    * The `msg` exceeds its group's max byte size
    * The `msg` is rejected because the store's remaining byte size is insufficient, and the store refuses to prune messages that are of higher priority than the inserted `msg`.

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { addMsg } = msgStore('http://127.0.0.1:8080')
const main = async () => {
    await addMsg(1, "Hello, world!").data // => { uuid: '1-2-3-4' }
}
```

---
`getMsg(<object> | <null>)`   

* `options` **object**
    * `options.uuid` **string?** An uuid of a message to request from the store.
    * `options.priority` **number?** The priority group to request a message from.
    * `options.reverse` **boolean?** Get from the back of the queue.

**Returns:**   
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?** The data returned if statusCode is 200
        * `data.uuid` **string** The uuid of the msg
        * `data.headers` **object** an object containing the headers sent with the message on insert
        * `data.msg` **string** the message   

Get a message from the store
* The `data` parameter will be null if no message is found

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { getMsg } = msgStore('http://127.0.0.1:8080')
const main = async () => {
    await getMsg({}) // => gets the next msg in the store
    await getMsg({ uuid: '1-2-3-4' }) // => gets the msg associated with uuid
    await getMsg({ priority: 1 }) // => get the next msg from priority 1
    await getMsg({ reverse: true }) // => gets the next msg in reverse order
    await getMsg({ priority: 2, reverse: true }) // => gets the next msg from group 2 in reverse order
}
```

---
`getNext()`   

**Returns:**   
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?** The data returned if statusCode is 200
        * `data.uuid` **string** The uuid of the msg
        * `data.headers` **object** an object containing the headers sent with the message on insert
        * `data.msg` **string** the message   

Get the next message from the store (the highest priority then oldest msg)
* The `data` parameter will be null if no message is found

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { getNext } = msgStore('http://127.0.0.1:8080')
const main = async () => {
    await getNext() // => gets the next msg in the store
}
```

---
`deleteMsg(<string>)`   
* `uuid` **string** The uuid to remove from the store   

**Returns:**   
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { addMsg, deleteMsg } = msgStore('http://127.0.0.1:8080')
const main = async () => {    
    const uuid = await addMsg(1, "Hello, world!").data.uuid // => '1-2-3-4'
    await deleteMsg({ uuid })
}
```

---
`getStream(<object> | <null>)`   

* `options` **object**
    * `options.uuid` **string?** An uuid of a message to request from the store.
    * `options.priority` **number?** The priority group to request a message from.
    * `options.reverse` **boolean?** Get from the back of the queue.

**Returns:**   
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?** The data returned if statusCode is 200
        * `data.uuid` **string** The uuid of the msg
        * `data.headers` **object** an object containing the headers sent with the message on insert
        * `data.msgStream` **ReadStream** the message in chunks   

Get a message from the store
* The `data` parameter will be null if no message is found

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { getStream } = msgStore('http://127.0.0.1:8080')
const main = async () => {    
    let result = result = await getStream({  })
    if (result.statusCode == 200) {
        let writer = createWriteStream('./my-file.json')
        await new Promise(resolve => {
            result.msgStream.pipe(writer)
            result.msgStream.on('close', () => {
                resolve()
            })
            result.msgStream.resume()
        })
    }
}
```

---
`addStream(<object>)`   

* `options` **object**
    * `options.priority` **number** the priority group to request a message from.
    * `options.saveToFile` **boolean?** Tell the server to save the msg to file.
    * `options.bytesizeOverride` **number?** The size of the file. (required if saveToFile is set to true.)
    * `options.msgStream` **ReadStream** The Readable stream to send to the store.

**Returns:**   
* `res` **object**
    * `res.statusCode` **number** The http status code
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?** The data returned if statusCode is 200
        * `data.uuid` **string** The uuid of the msg
        * `data.headers` **object** An object containing the headers sent with the message on insert
        * `data.msgStream` **ReadStream** The message in chunks   

Send a message stream to the store
* Returns `400` if received a bad request.
* Returns `403` if attempting to save to file if the server file storage in not configured.
* Returns `409` if:   
    * The `msg` exceeds the store max byte size
    * The `msg` exceeds its group's max byte size
    * The `msg` is rejected because the store's remaining byte size is insufficient, and the store refuses to prune messages that are of higher priority than the inserted `msg`.

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { addStream } = msgStore('http://127.0.0.1:8080')
const main = async () => {    
    let reader = createReadStream('./tests/test-file.json')
    let options = {
        priority: 1, 
        saveToFile: true, 
        bytesizeOverride: 1, 
        fileName: 'file-sent.json', 
        msgStream: reader
    }
    await addStream(options)        
}
```

---
`getGroup(<object>)`   
* `options` **object**   
    * `options.priority` **number?** the priority group to get infomation of.
    * `options.includeMsgData` **boolean?** Include a list msg uuids and byte sizes.   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200.
    * `res.data` **object[]?** The data returned if statusCode is 200.
        * `data.priority` **number** The priority of the group.
        * `data.byteSize` **number** The byte size of the group.
        * `data.maxByteSize` **number?** The max byte size of the group.
        * `data.msgCount` **number** The number of messages in the group.
        * `data.messages` **object[]**
            * `messages.uuid` **string** The uuid of a message belonging to the group.
            * `messages.byteSize` **number** The byte size of the message in the group.

**Examples**
```javascript
import msgStoreClient from 'msg-store-http-client'
const { getGroup } = msgStore('http://127.0.0.1:8080')
const main = async () => {    
    const groups = await getGroup({}).data // => [ group1, group2, ...other groups ]
    const group1 = await getGroup({ priority: 1 }).data // => [ groupData1 ]
}
```

---
`deleteGroup(<number>)`   
* `priority` **number** The priority group to remove from the store   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200

Delete an entire group of messages in the store

---
`setGroupDefaults(<number>, <object>)`
* `priority` **number** The priority group defaults to modify.  
* `options` **object** 
    * `options.maxByteSize` **number** The max capacity of the store in bytes.   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200

Change a group's max byte size

---
`getGroupDefaults(<number>)`   
* `priority` **number** The priority group to get the defaults for   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?**
        * `res.data.maxByteSize` **number** The max byte size of the group

Get a groups max byte size

---
`deleteGroupDefaults(<number>)`   
* `priority` **number** The priority group to remove from the store   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200

Delete a group's default behavior

---
`getStats()`   
**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?**
        * `data.inserted` **number** The number of messages inserted.
        * `data.deleted` **number** The number of messages deleted via the api.
        * `data.pruned` **number?** The number of messages pruned automatically.

Get the current value of the store's statistics.

---
`updateStats(<object>)`   
* `options` **object?**
    * `options.add` **boolean** Determine if the new stats should be added to the current stats. Default: `false`.
    * `options.inserted` **number** The number of messages inserted.
    * `options.deleted` **number** The number of messages deleted via the api.
    * `options.pruned` **number?** The number of messages pruned automatically.

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?**
        * `data.inserted` **number** The number of messages inserted.
        * `data.deleted` **number** The number of messages deleted via the api.
        * `data.pruned` **number?** The number of messages pruned automatically.

Change the statistics of the store, returning the its last value.

---
`deleteStats()`   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?**
        * `data.inserted` **number** The number of messages inserted.
        * `data.deleted` **number** The number of messages deleted via the api.
        * `data.pruned` **number?** The number of messages pruned automatically.


Reset the statistics of the store, returning its last value.

---
`getStore()`   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200
    * `res.data` **object?**
        * `data.byteSize` **number** 
        * `data.maxByteSize` **number?** 
        * `data.msgCount` **number** 
        * `data.groupCount` **number** 
        * `data.groups` **object[]**   
            * `data.groups.priority` **number**   
            * `data.groups.byteSize` **number**   
            * `data.groups.maxByteSize` **number?**   
            * `data.groups.msgCount` **number**   
        * `data.groupDefaults` **object[]** 
            * `data.groupDefaults.priority` **number**
            * `data.groupDefaults.maxByteSize` **number?**


Get information about the state of the store.

---
`updateStore(<object>)`
* `options` **object** 
    * `options.maxByteSize` **number** The max capacity of the store in bytes   

**Returns:**
* `res` **object**
    * `res.statusCode` **number** The http status code.
    * `res.error` **string?** The error message if statusCode is not 200

Change the store's max byte size
