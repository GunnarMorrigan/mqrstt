<div align="center">

# `📟 MQRSTT`

[![Crates.io](https://img.shields.io/crates/v/mqrstt.svg)](https://crates.io/crates/mqrstt)
[![Docs](https://docs.rs/mqrstt/badge.svg)](https://docs.rs/mqrstt)
[![dependency status](https://deps.rs/repo/github/GunnarMorrigan/mqrstt/status.svg)](https://deps.rs/repo/github/GunnarMorrigan/mqrstt)
[![codecov](https://codecov.io/github/GunnarMorrigan/mqrstt/branch/main/graph/badge.svg)](https://app.codecov.io/gh/GunnarMorrigan/mqrstt)

`MQRSTT` is an MQTTv5 client that provides sync and async (smol and tokio) implementation.

Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
The stream has to implement the smol or tokio [`AsyncReadExt`] and [`AsyncWrite`] traits.

</div>

## Features
- MQTT v5
- Runtime agnostic (Smol, Tokio)
- Sync
- TLS/TCP
- Lean
- Keep alive depends on actual communication
- This tokio implemention has been fuzzed using cargo-fuzz!


  
### To do
- Even More testing
- Add TLS examples to repository

## MSRV
From 0.3 the tokio and smol variants will require MSRV: 1.75 due to async fn in trait feature.

## TCP & TLS Examples

### Notes:
- Your handler should not wait too long
- Create a new connection when an error or disconnect is encountered
- Handlers only get incoming packets


### Smol example:
```rust
use mqrstt::{
    packets::{self, Packet},
    AsyncEventHandler, MqttClient, NetworkBuilder, NetworkStatus,
};
pub struct PingPong {
    pub client: MqttClient,
}
impl AsyncEventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    async fn handle(&mut self, event: packets::Packet) {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client.publish(p.topic.clone(), p.qos, p.retain, b"pong").await.unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            }
            Packet::ConnAck(_) => {
                println!("Connected!")
            }
            _ => (),
        }
    }
}
fn main() {
    smol::block_on(async {
        let (mut network, client) = NetworkBuilder::new_from_client_id("mqrsttSmolExample").smol_network();
        let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        network.connect(stream, &mut pingpong).await.unwrap();

        // This subscribe is only processed when we run the network
        client.subscribe("mqrstt").await.unwrap();

        let task_handle = smol::spawn(async move {
            let result = network.run(&mut pingpong).await;
            (result, pingpong)
        });

        smol::Timer::after(std::time::Duration::from_secs(30)).await;
        client.disconnect().await.unwrap();

        let (result, _pingpong) = task_handle.await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), NetworkStatus::OutgoingDisconnect);
    });
}

```

### Tokio example:
```rust
use mqrstt::{
    packets::{self, Packet},
    AsyncEventHandler, MqttClient, NetworkBuilder, NetworkStatus,
};
use tokio::time::Duration;

pub struct PingPong {
    pub client: MqttClient,
}
impl AsyncEventHandler for PingPong {
    // Handlers only get INCOMING packets.
    async fn handle(&mut self, event: packets::Packet) {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client.publish(p.topic.clone(), p.qos, p.retain, b"pong").await.unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            }
            Packet::ConnAck(_) => {
                println!("Connected!")
            }
            _ => (),
        }
    }
}

#[tokio::main]
async fn main() {
    let (mut network, client) = NetworkBuilder::new_from_client_id("TokioTcpPingPongExample").tokio_network();

    let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883)).await.unwrap();
    let stream = tokio::io::BufStream::new(stream);

    let mut pingpong = PingPong { client: client.clone() };

    network.connect(stream, &mut pingpong).await.unwrap();

    client.subscribe("mqrstt").await.unwrap();

    let network_handle = tokio::spawn(async move {
        let result = network.run(&mut pingpong).await;
        (result, pingpong)
    });

    tokio::time::sleep(Duration::from_secs(30)).await;
    client.disconnect().await.unwrap();

    let (result, _pingpong) = network_handle.await.unwrap();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), NetworkStatus::OutgoingDisconnect);
}

```

### Sync example:
```rust
use mqrstt::{
    MqttClient,
    ConnectOptions,
    new_sync,
    packets::{self, Packet},
    EventHandler,
    sync::NetworkStatus,
};
use std::net::TcpStream;
use bytes::Bytes;

pub struct PingPong {
    pub client: MqttClient,
}

impl EventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    fn handle(&mut self, event: packets::Packet) {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client
                            .publish_blocking(
                                p.topic.clone(),
                                p.qos,
                                p.retain,
                                Bytes::from_static(b"pong"),
                            ).unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            },
            Packet::ConnAck(_) => { println!("Connected!") },
            _ => (),
        }
    }
}


let mut client_id: String = "SyncTcpPingReqTestExample".to_string();
let options = ConnectOptions::new(client_id);

let address = "broker.emqx.io";
let port = 1883;

let (mut network, client) = new_sync(options);

// IMPORTANT: Set nonblocking to true! No progression will be made when stream reads block!
let stream = TcpStream::connect((address, port)).unwrap();
stream.set_nonblocking(true).unwrap();

let mut pingpong = PingPong {
    client: client.clone(),
};

network.connect(stream, &mut pingpong).unwrap();

let res_join_handle = std::thread::spawn(move ||
    loop {
        match network.poll(&mut pingpong) {
            Ok(NetworkStatus::ActivePending) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            },
            Ok(NetworkStatus::ActiveReady) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            },
            otherwise => return otherwise,
        }
    }
);

std::thread::sleep(std::time::Duration::from_secs(30));
client.disconnect_blocking().unwrap();
let join_res = res_join_handle.join();
assert!(join_res.is_ok());
let res = join_res.unwrap();
assert!(res.is_ok());
```

## FAQ
  - Not much gets frequently asked, so please do! :)
  - Open to feature requests

## License
Licensed under
* Mozilla Public License, Version 2.0, [(MPL-2.0)](https://choosealicense.com/licenses/mpl-2.0/)

## Contribution
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, shall be licensed under MPL-2.0, without any additional terms or
conditions.
