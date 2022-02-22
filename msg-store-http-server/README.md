# msg-store-http-server
An http server to expose the msg-store API for broader applications

# Getting Started
```bash
$ cargo install msg-store-http-server
$ msg-store-http-server
```
or with option flags
```bash
$ msg-store-http-server --node-id=12345 --database=leveldb --leveldb-path=/path/to/leveldb/dir --file-storage-path=/path/to/filestorage/dir
```

or with a default config file at $HOME/.msg-store/config.json
```bash
$ msg-store-http-server -c
```

or with a config file with a custom path
```bash
$ msg-store-http-server --config-path=/path/to/config/file
```

The default configuration file create with the -c flag at $HOME/.msg-store/config.json
```json
{
  "host": "127.0.0.1",
  "port": 8080,
  "node_id": null,
  "database": "mem",
  "leveldb_path": null,
  "file_storage": false,
  "file_storage_path": null,
  "max_byte_size": null,
  "groups": null,
  "no_update": null,
  "update": true
}
```

## Node ID
The node ID can be any unsigned 32 bit integer and defaults to 0.
```
$ msg-store-http-server --node-id=12345
```

## Database
The default database is the included in-memory database.   
To use leveldb, pass in the database flag to app or change the databasee property in the config.json file. This will create a leveldb directory in the $HOME/.msg-store directory.
```
$ msg-store-http-server --database=leveldb
```
To set a custom path to leveldb use the --leveldb-path flag or change the leveldb_path property in the config.json. A database will be created if one does not exist. The leveldb plugin uses 2 leveldb instances, so the path should be a directory.
```
$ msg-store-http-server --database=leveldb --leveldb-path=/path/to/leveldb/dir
```

## File Storage
To use the file storage feature for large messages, pass the --file-storage flag or set the file_storage property to true in the config.json. This will create a file storage directory in the $HOME/.msg-store directory.
```
$ msg-store-http-server --file-storage
```
To set a custom path, pass the --file-storage-path flag or set the file_storage_path property to the desired path in the config.json.
```
$ msg-store-http-server --file-storage --file-storage-path=/path/to/file/storage/dir
```
This path should also be a directory.

## Host & Port
Set the host and port with their respective flags or change them in the config.json file.
```
$ msg-store-http-server --host=my.host.com --port=3300
```
The defaults are 127.0.0.1 and 8080

## Store & Group Defaults
The store and priority group defaults can be changed on on the fly and will be reflected in the config.json (if the server is set to use one). If the server should not reflect these changes pass the --no-update flag or set the no_update property to false in the config.json.
Note that --update and --no-update are conflicting options and the app will reject the setting.

## Max Bytesizes
The max bytesize of the store or any of the priority groups cannot be passed via flags, but can be configured in the config.json file or via the http api.

## Available Clients
[msg-store-http-client](https://www.npmjs.com/package/msg-store-http-client)