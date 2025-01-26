# RPC Server

## Introduction
Neptune core implements an RPC server and client based on [tarpc](file:///Users/monordi/apps/neptune-core/target/doc/tarpc/index.html). The RPC server listens on a TCP port and accepts json serialized requests, the default port is `. The RPC server is used to query the state of the blockchain, submit transactions, and perform other operations.

It is presently easiest to create a tarpc client in rust. To do so, one should add neptune-cash as a dependency and then do something like:

```rust
use neptune_cash::rpc_server::RPCClient;
use neptune_cash::rpc_auth;
use tarpc::tokio_serde::formats::Json;
use tarpc::serde_transport::tcp;
use tarpc::client;
use tarpc::context;


// create a serde/json transport over tcp.
let transport = tcp::connect("127.0.0.1:9799", Json::default).await.unwrap();

// create an rpc client using the transport.
let client = RPCClient::new(client::Config::default(), transport).spawn();

// query neptune-core server how to find the cookie file
let cookie_hint = client.cookie_hint(context::current()).await.unwrap().unwrap();

// load the cookie file from disk and assign it to a token.
let token: rpc_auth::Token = rpc_auth::Cookie::try_load(&cookie_hint.data_directory).await.unwrap().into();

// query any RPC API, passing the auth token.  here we query block_height.
let block_height = client.block_height(context::current(), token).await.unwrap().unwrap();
```



For other languages, one would need to connect to the RPC TCP port and then manually construct the appropriate json method call. Examples of this will be forthcoming in the future.

See rpc_auth for descriptions of the authentication mechanisms.

Every RPC method returns an RpcResult which is wrapped inside a tarpc::Response by the rpc server.

### Versioning 

The RPC server supports versioning. The version of the RPC server is returned by the `version` method. The version is a string in the format `major.minor.patch`.

### RPC consistency guarentees 


### Authentication mechanisms

The RPC server supports two authentication mechanisms: cookie and token. The cookie mechanism is used to authenticate a user by a file on disk. The token mechanism is used to authenticate a user by a token string.

#### Cookie authentication

#### Token authentication

#### Digest authentication 

Coming soon.

## RPC Methods

