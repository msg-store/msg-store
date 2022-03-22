import msgStore from "../index.js"
import { createHash } from 'crypto'
import { exec } from 'child_process'
import { createReadStream, createWriteStream, rmSync, mkdirSync, existsSync, unlinkSync } from 'fs'
import { Readable } from 'stream'
// import { eventNames, listenerCount } from "process"
const { 
    addMsg, getMsg, getNext, deleteMsg, 
    getGroup, deleteGroup,
    getStats, updateStats, deleteStats,
    getStore, updateStore,
    getGroupDefaults, setGroupDefaults, deleteGroupDefaults,
    getStream, addStream,
    // exportMsgs
} = msgStore("http://127.0.0.1:8080")

/* 
    create tmp folder
    make a thousand files
    get a hash of each file
    send the files to the server
    get all files one at a time
    compare each hash    
    remove temp folder
*/

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

function getRandomIntInclusive(min, max) {
    min = Math.ceil(min);
    max = Math.floor(max);
    return Math.floor(Math.random() * (max - min + 1) + min); //The maximum is inclusive and the minimum is inclusive
}

function createContents(kb, stream) {
    const characters = 'qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890!@#$%^&*()~`[]{};?><,./|'
    const characterLength = characters.length
    // console.log(stream)
    for (let i = 0; i < kb; i++) {
        let word = ''
        for (let c = 0; c < 1023; c++) {
            word = `${word}${characters[getRandomIntInclusive(0, characterLength - 1)]}`
        }
        word = `${word} `
        stream.write(word)
    }
}

function createFileName() {
    const characters = 'qwertyuiopasdfghjklzxcvbnmQWERTYUIOPASDFGHJKLZXCVBNM1234567890'
    const characterLength = characters.length
    let word = ''
    for (let c = 0; c < 30; c++) {
        word = `${word}${characters[getRandomIntInclusive(0, characterLength - 1)]}`
    }
    return word.trim()
}

async function createFile() {
    const fileName = createFileName()
    const file = createWriteStream(`/tmp/msg-store-test/source-files/${fileName}`)
    let untilDone = new Promise((resolve) => {
        file.on('close', () => {
            resolve()
        })
    })
    // console.log(file)
    createContents(10 * 1000, file)
    file.end()
    await untilDone
    return fileName
}

async function getFileHash(dir, fileName) {
    const filePath = `/tmp/msg-store-test/${dir}/${fileName}`
    return getHash(createReadStream(filePath))
}

async function sendFile(fileName, fileSize) {
    let reader = createReadStream(`/tmp/msg-store-test/source-files/${fileName}`)
    let result = await addStream({ priority: 1, saveToFile: true, bytesizeOverride: fileSize, fileName, msgStream: reader })
}

async function createServer() {
    let server
    await new Promise((resolve, _reject) => {
        const bin = '../../../target/debug/msg-store-http-server'
        // const bin = 'msg-store-server'
        const leveldbPath = '/tmp/msg-store-test/leveldb'
        const fileStoragePath = '/tmp/msg-store-test/filestorage'
        console.log('starting server...')
        server = exec(`${bin} --node-id=12345 --database=leveldb --leveldb-path=${leveldbPath} --file-storage-path=${fileStoragePath}`)
        server.stdout.on('data', msg => {
            // console.log(msg)
            if (msg.includes('Starting "actix-web-service-127.0.0.1:8080" service on 127.0.0.1:8080')) {
                resolve()
            }
        })
        server.stderr.on('data', msg => {
            process.stdout.write(`error: ${msg}`)
            // console.log(msg)
        })
        server.stdout.on('data', msg => {
            // process.stdout.write(`${msg}\n`)
            // console.log(msg)
            // if (msg.includes('400')) {
            //     console.log(msg)
            // }
        })
    })
    console.log('server started')
    return server
}

async function removeFile(filePath) {
    unlinkSync(filePath, { force: true, recursive: true })
}

// async function createFile() {
    
// }

async function send1000messages() {
    // rmSync('/tmp/msg-store-test/', { force: true, recursive: true })
    // mkdirSync('/tmp/msg-store-test/')
    // const server = await createServer()
    console.log(await getStore())
    for (let i = 0; i < 20_000; i++) {
        if (i % 1000 === 0) {
            process.stdout.write(`sending message ${i}\n`)
        }
        await Promise.all([
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message"),
            addMsg(1, "my message")
        ])
        // console.log(eventNames())
        // await addMsg(1, "my message")
        // process.stdout.write(`sending message ${i}`);
        // process.stdout.clearLine(0);
        // process.stdout.cursorTo(0);
    }
    // process.stdout.write("\n");
    // let results = await Promise.all(msgs)
    // server.kill()
    // console.log(results)
}

async function send1000Files() {
    const iterations = 1000;
    // rmSync('/tmp/msg-store-test/', { force: true, recursive: true })
    // mkdirSync('/tmp/msg-store-test/')
    mkdirSync('/tmp/msg-store-test/source-files')
    const createFilePromises = []
    console.log('creating files...')
    for (let i = 0; i < iterations; i++) {
        createFilePromises.push(createFile())
    }
    const fileNames = await Promise.all(createFilePromises)
    console.log('finished creating files.')
    // console.log(fileNames)

    // const server = await createServer()
    
    // let count = 0
    // const readers = []
    for (let i = 0; i < iterations; i++) {
        // const sendFiles = []
        // for (let x = 0; x < 10; x++) {
        //     readers[count] = createReadStream(`/tmp/msg-store-test/source-files/${fileNames[count]}`)
        //     sendFiles.push()
        //     count++
        // }
        // await Promise.all(sendFiles)
        const reader = createReadStream(`/tmp/msg-store-test/source-files/${fileNames[i]}`)
        await addStream({ priority: 1, saveToFile: true, bytesizeOverride: 1, fileName: fileNames[i], msgStream: reader })
    }
    
    // for (let i = 0; i < results.length; i++) {
    //     results[i] = results[i].uuid
    // }
    // console.log(results)
    // console.log((await getStore()).data)
    // let fileName = await createFile()
    // let hash = await getFileHash('source-files', fileName)
    // console.log('fileName: ', fileName)
    // console.log('hash: ', hash)
    // server.kill()

    // const removeAllSourceFiles = []
    // console.log('removing files...')
    // for (let i = 0; i < fileNames.length; i++) {
    //     console.log(`removing ${fileNames[i]}...`)
    //     removeAllSourceFiles.push(removeFile(`/tmp/msg-store-test/source-files/${fileNames[i]}`))
    // }
    // await Promise.all(removeAllSourceFiles)

    // rmSync(`/tmp/msg-store-test/`, { force: true, recursive: true })
      
}

async function main () {

    await send1000messages()
    // await send1000Files()

    // while (true) {

    // }

}

// console.log(createContents(100, 20))

main()