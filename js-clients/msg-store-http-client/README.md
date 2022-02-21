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

    const msg = await getMsg({ uuid }).data // => "hello, world!"

}

main()

```