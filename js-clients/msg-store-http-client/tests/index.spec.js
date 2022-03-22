import msgStore from "../index.js"
import { assert, expect } from "chai"
import { Readable } from 'stream'
import { createReadStream, createWriteStream, rmSync, mkdirSync, existsSync, unlinkSync } from 'fs'
import { exec } from 'child_process'
import { createHash } from 'crypto'
// import * as path from 'path'

const { 
    addMsg, getMsg, getNext, deleteMsg, 
    getGroup, deleteGroup,
    getStats, updateStats, deleteStats,
    getStore, updateStore,
    getGroupDefaults, setGroupDefaults, deleteGroupDefaults,
    getStream, addStream,
    // exportMsgs
} = msgStore("http://127.0.0.1:8080")

let server
const msgStorePath = './msg-store-test'
const leveldbPath = `${msgStorePath}/leveldb`
const fileStoragePath = `${msgStorePath}/filestorage`
beforeEach(async function () {
    // TODO: change to reflect file location
    rmSync(msgStorePath, { force: true, recursive: true })
    mkdirSync(msgStorePath)
    await new Promise((resolve, _reject) => {
        const bin = '../../target/debug/msg-store-http-server'
        // const bin = 'msg-store-server'
        
        server = exec(`${bin} --node-id=12345 --database=leveldb --leveldb-path=${leveldbPath} --file-storage-path=${fileStoragePath}`)
        server.stdout.on('data', msg => {
            if (msg.includes('Starting "actix-web-service-127.0.0.1:8080" service on 127.0.0.1:8080')) {
                resolve()
            }
        })
        server.stdout.on('data', msg => {
            // console.log(msg)
            // if (msg.includes('400')) {
            //     console.log(msg)
            // }
        })
    })
})
afterEach(async function () {
    // TODO: change to reflect file location
    rmSync(msgStorePath, { force: true, recursive: true })
    server.kill()
})
const getHash = reader => {
    return new Promise(resolve => {
        let hash = createHash('sha256')
        reader.on('data', chunk => {
            hash.write(chunk)
        }).on('close', () => {
            hash.end()
            resolve(hash.digest('hex'))
        })
    })
}
describe("messaging", function () {
    it("should add a message, get the same message and then delete that message", async function () {
        const msg = "hello, world"
        let result = await addMsg(1, msg)
        expect(result.statusCode).to.eql(200)
        let uuid = result.data.uuid
        assert.isString(uuid)
        // console.log(uuid)
        result = await getMsg({ uuid })
        expect(result.statusCode).to.eql(200)
        result = await deleteMsg(uuid)
        expect(result.statusCode).to.eql(200)
        result = await getMsg({ uuid })
        expect(result.statusCode).to.eql(200)
        assert.isUndefined(result.data.msg)
    })
    it('should insert multiple message and get them out in different orders', async function () {
        const [ r0, _r1, _r2, _r3, _r4, r5, r6, _r7, _r8, _r9 ] = [
            await addMsg(1, 'message 0'),
            await addMsg(1, 'message 1'),
            await addMsg(1, 'message 2'),
            await addMsg(1, 'message 3'),
            await addMsg(1, 'message 4'),
            await addMsg(1, 'message 5'),
            await addMsg(2, 'message 6'),
            await addMsg(2, 'message 7'),
            await addMsg(2, 'message 8'),
            await addMsg(2, 'message 9')
        ]        
        let result = await getMsg({ priority: 1 })
        expect(result.data.uuid).to.eql(r0.data.uuid)
        result = await getMsg({ priority: 1, reverse: true })
        expect(result.data.uuid).to.eql(r5.data.uuid)
        result = await getNext()
        expect(result.data.uuid).to.eql(r6.data.uuid)
        result = await getMsg()
        expect(result.data.uuid).to.eql(r6.data.uuid)
    })
})
describe('group routes', function () {
    it('should get group data after msg insert and then delete group', async function () {
        await addMsg(1, 'foo')
        await addMsg(2, 'foo')
        let result = await getGroup({ priority: 1, includeMsgData: true })
        expect(result.data.length).to.eql(1)
        expect(result.data[0].messages.length).to.eql(1)
        result = await getGroup()
        expect(result.data.length).to.eql(2)
        expect(result.data[0].messages.length).to.eql(0)
        result = await getGroup({ priority: 1 })
        expect(result.data.length).to.eql(1)
        expect(result.data[0].messages.length).to.eql(0)
        await deleteGroup(1)
        result = await getGroup({ priority: 1 })
        expect(result.data.length).to.eql(0)
    })
})
describe('stat routes', function () {
    it('should update on every msg action', async function () {
        let result = await getStats()
        expect(result.data).to.eql({ inserted: 0, deleted: 0, pruned: 0 })
        let uuid = await (await addMsg(1, 'foo')).data.uuid
        result = await getStats()
        expect(result.data).to.eql({ inserted: 1, deleted: 0, pruned: 0 })
        await deleteMsg(uuid)
        result = await getStats()
        expect(result.data).to.eql({ inserted: 1, deleted: 1, pruned: 0 })
        await addMsg(1, 'bar')
        await updateStore({ maxByteSize: 2 })
        result = await getStats()
        expect(result.data).to.eql({ inserted: 2, deleted: 1, pruned: 1 })
        await updateStats({ inserted: 10, deleted: 10, pruned: 10 })
        result = await getStats()
        expect(result.data).to.eql({ inserted: 10, deleted: 10, pruned: 10 })
        await updateStats({ inserted: 10, deleted: 10, pruned: 10, add: true })
        result = await getStats()
        expect(result.data).to.eql({ inserted: 20, deleted: 20, pruned: 20 })
        result = await deleteStats()
        expect(result.data).to.eql({ inserted: 20, deleted: 20, pruned: 20 })
        result = await getStats()
        expect(result.data).to.eql({ inserted: 0, deleted: 0, pruned: 0 })
    })
})

describe('store routes', function () {
    it('should get store defaults, update them and get them again', async function () {
        let result = await getStore()
        expect(result.data).to.eql({
            byteSize: 0,
            maxByteSize: null,
            msgCount: 0,
            groupCount: 0,
            groups: [],
            groupDefaults: []
        })
        await addMsg(1, 'foo')
        result = await getStore()
        expect(result.data.byteSize).to.eql(3)
        await updateStore({ maxByteSize: 10 })
        result = await getStore()
        expect(result.data).to.eql({
            byteSize: 3,
            maxByteSize: 10,
            msgCount: 1,
            groupCount: 1,
            groups: [ { priority: 1, byteSize: 3, maxByteSize: null, msgCount: 1 } ],
            groupDefaults: []
        })
        await updateStore({ maxByteSize: 2 })
        result = await getStore()
        expect(result.data).to.eql({
            byteSize: 0,
            maxByteSize: 2,
            msgCount: 0,
            groupCount: 0,
            groups: [],
            groupDefaults: []
        })
        // TODO: test for group defaults
    })
})
describe('group defaults routes', function () {
    it('should add a group default, get it, update it, and deleted it', async function () {
        let result = await (await getGroupDefaults()).data
        expect(result.length).to.eql(0)
        await setGroupDefaults(1, { maxByteSize: 10 })
        await setGroupDefaults(2, { maxByteSize: 20 })
        result = await (await getGroupDefaults()).data
        expect(result[0].maxByteSize).to.eql(10)
        expect(result[1].maxByteSize).to.eql(20)
        result = await ( await getGroupDefaults(1) ).data[0].priority
        expect(result).to.eql(1)
        await addMsg(1, '123456')
        await setGroupDefaults(1, { maxByteSize: 5 })
        result = await ( await getStats() ).data.pruned
        expect(result).to.eql(1)
        await deleteGroupDefaults(1)
        result = await ( await getGroupDefaults() ).data
        expect(result.length).to.eql(1)
        expect(result[0].priority).to.eql(2)
        // console.log(result)
    })
})

describe('streaming', function () {
    this.beforeEach(async function () {
        if (existsSync("./tests/test-file-result.json")) {
            unlinkSync('./tests/test-file-result.json')
        }
    })
    this.afterEach(async function () {
        if (existsSync("./tests/test-file-result.json")) {
            unlinkSync('./tests/test-file-result.json')
        }
    })
    it('should stream messages back and forth', async function () {
        // Read a message into a stream
        // In this example the readable represents a file name "my-file.json"
        let reader = new Readable({
            read(size) { }
        })
        // Push the contents of the file into our read stream
        const fileName = 'my-file.json'
        const fileContents = 'my file contents'
        reader.push(fileContents)
        reader.push(null)
        // Feed the read stream into the http write stream 
        // to send to the server and wait for the response
        let result = await addStream({ priority: 4, saveToFile: true, bytesizeOverride: fileContents.length, fileName, msgStream: reader })
        expect(result.statusCode).to.eql(200)
        assert.isString(result.uuid)
        // Prepare a msg string for reading a message from the server
        let msg = ''
        // Send the request and wait for the result
        result = await getStream({  })
        let { uuid, statusCode, msgStream, ...headers } = result
        await new Promise (resolve => {
            // Read from stream into the msg string (or to a file)
            msgStream.on('data', chunk => {
                if (chunk !== null) {
                    msg += chunk
                }
                // console.log('chunk: %s#done', chunk.toString())
                
            }).on('end', () => {
                resolve()
            })
            msgStream.resume()
        })        
        // we should receive the whole msg and also the headers including the file name
        expect(uuid).to.eql(result.uuid)
        expect(msg).to.eql(fileContents)
        expect(headers.fileName).to.eql(fileName)
    })
    it('should stream a very large file', async function () {
        this.timeout(5000);
        let reader = createReadStream('./tests/test-file.json')
        let result = await addStream({ priority: 1, saveToFile: true, bytesizeOverride: 1, fileName: 'file-sent.json', msgStream: reader })
        expect(result.statusCode).to.eql(200)
        assert.isString(result.uuid)
        let writer = createWriteStream('./tests/test-file-result.json')
        result = await getStream({  })
        expect(result.statusCode).to.eql(200)
        expect(result.fileName).to.eql('file-sent.json')
        await new Promise(resolve => {
            result.msgStream.pipe(writer)
            result.msgStream.on('close', () => {
                resolve()
            })
            result.msgStream.resume()
        })
        // check the hash of the files to validate that they are the save
        let [ testFileHash, testFileResultHash ] = await Promise.all([
            getHash(createReadStream('./tests/test-file.json')),
            getHash(createReadStream('./tests/test-file-result.json'))
        ])
        expect(testFileResultHash).to.eql(testFileHash)        
    })
})
// describe('exporting', function () {
//     // const startBackupServer = async () => {
//     //     await new Promise((resolve, _reject) => {
//     //         const bin = '../msg-store-server/target/debug/msg-store-server'
//     //         // const bin = 'msg-store-server'
//     //         const leveldbPath = '/tmp/msg-store-test/leveldb'
//     //         const fileStoragePath = '/tmp/msg-store-test/filestorage'
//     //         backupServer = exec(`${bin} --node-id=12345 --database=leveldb --leveldb-path=${leveldbPath} --file-storage-path=${fileStoragePath}`)
//     //         backupServer.stdout.on('data', msg => {
//     //             if (msg.includes('Starting "actix-web-service-127.0.0.1:8080" service on 127.0.0.1:8080')) {
//     //                 resolve()
//     //             }
//     //         })
//     //         backupServer.stdout.on('data', msg => {
//     //             // console.log(msg)
//     //             // if (msg.includes('400')) {
//     //             //     console.log(msg)
//     //             // }
//     //         })
//     //     })
//     // }
//     // let backupServer
//     // beforeEach(async function () {
//     //     rmSync('/tmp/msg-store-test-export', { force: true, recursive: true })
//     //     rmSync('/tmp/msg-store-test-export', { force: true, recursive: true })
//     //     mkdirSync('/tmp/msg-store-test-export')
        
//     // })
//     // afterEach(async function () {
//     //     // TODO: change to reflect file location
//     //     rmSync('/tmp/msg-store-test-export', { force: true, recursive: true })
//     //     if (backupServer) {
//     //         backupServer.kill()
//     //     }
        
//     // })
//     it.ignore('should export messages into a leveldb file', async function () {
//         this.timeout(5000);
//         let reader = createReadStream('./tests/test-file.json')
//         let result = await addStream({ priority: 1, saveToFile: true, byteSizeOverride: 1, fileName: 'file-sent.json', msgStream: reader })
//         expect(result.statusCode).to.eql(200)
//         assert.isString(result.uuid)

//         // export data to backup location
//         result = await exportMsgs('/tmp/msg-store-test-export')
//         console.log(result.statusCode)

//         // start backup msg-store server
//         // backupServer = await startBackupServer();
//         // expect(result.statusCode).to.eql(200)

//         // let writer = createWriteStream('./tests/test-file-result.json')
//         // result = await getStream({  })
//         // expect(result.statusCode).to.eql(200)
//         // expect(result.fileName).to.eql('file-sent.json')
//         // await new Promise(resolve => {
//         //     result.msgStream.pipe(writer)
//         //     result.msgStream.on('close', () => {
//         //         resolve()
//         //     })
//         //     result.msgStream.resume()
//         // })
//         // // check the hash of the files to validate that they are the save
//         // let [ testFileHash, testFileResultHash ] = await Promise.all([
//         //     getHash(createReadStream('./tests/test-file.json')),
//         //     getHash(createReadStream('./tests/test-file-result.json'))
//         // ])
//         // expect(testFileResultHash).to.eql(testFileHash)   
//     })
// })

// TODO: test that the config is being updated on change