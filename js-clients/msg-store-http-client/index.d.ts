import { Readable } from 'stream'

interface Headers {
    [key: string]: string
}

interface Response {
    statusCode: number,
    error?: string
}

interface AddMsgResult extends Response {
    statusCode: 200 | 400 | 403 | 409,
    data?: {
        uuid: string
    }
}

interface GetMsgOptions {
    uuid?: string,
    priority?: number,
    reverse?: boolean
}

interface GetResultData {
    uuid: string,
    headers: Headers,
    msg: string
}

interface GetMsgResult extends Response {
    statusCode: 200 | 400
    data?: GetResultData
}

interface GetStreamResultData {
    uuid: string,
    headers: Headers,
    msgStream: Readable
}

interface GetStreamResult extends Response {
    statusCode: 200 | 400
    data?: GetStreamResultData
}

interface GetGroupOptions {
    priority?: number,
    includeMsgData?: boolean
}

interface GroupMsgData {
    uuid: string,
    byteSize: number
}

interface Group {
    priority: number,
    byteSize: number,
    maxByteSize?: number,
    msgCount: number,
    messages: GroupMsgData[]
}

interface GetGroupResult extends Response {
    data?: []
}

interface SetGroupDefaultsOptions {
    maxByteSize?: number
}

interface GroupDefaults {
    maxByteSize?: number
}

interface GetGroupDefaultsResult extends Response {
    data?: GroupDefaults[]
}

interface Stats {
    inserted: number,
    deleted: number,
    pruned: number
}

interface GetStatsResult extends Response {
    data?: Stats
}

interface UpdateStatsOptions {
    add?: boolean,
    inserted?: number,
    deleted?: number,
    pruned?: number
}

interface GroupData {
    priority: number,
    byteSize: number,
    maxByteSize: number,
    msgCount: number
}

interface GroupDefaultsPri {
    priority: number,
    maxByteSize?: number
}

interface GetStoreResult {
    data?: {
        byteSize: number,
        maxByteSize?: number,
        msgCount: number,
        groupCount: number,
        groups: GroupData[],
        groupDefaults: GroupDefaultsPri[]
    }    
}

interface UpdateStoreOptions {
    maxByteSize?: number
}

interface MsgStoreAPI {
    addMsg: (priority: number, msg: string) => Promise<AddMsgResult>,
    getMsg: (options: GetMsgOptions) => Promise<GetMsgResult>,
    getNext: () => Promise<GetMsgResult>,
    deleteMsg: (uuid: string) => Promise<Response>,
    getStream: (options: GetMsgOptions) => Promise<GetStreamResult>,
    addStream: (priority: number, msgStream: Readable) => Promise<AddMsgResult>,
    getGroup: (priority: number | null, options: GetGroupOptions) => Promise<GetGroupResult>,
    deleteGroup: (priority: number) => Promise<Response>,
    setGroupDefaults: (priority: number, options: SetGroupDefaultsOptions) => Promise<Response>,
    getGroupDefaults: (priority: number) => Promise<GetGroupDefaultsResult>,
    deleteGroupDefaults: (priority: number) => Promise<Response>,
    getStats: () => Promise<GetStatsResult>,
    updateStats: (options: UpdateStatsOptions) => Promise<Response>,
    deleteStats: () => Promise<Response>,
    getStore: () => Promise<GetStoreResult>,
    updateStore: (options: UpdateStoreOptions) => Promise<Response>
}

export default function (url: string): MsgStoreAPI;