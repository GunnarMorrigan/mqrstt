<div align="center">

# `📟 MQRSTT`

[![Crates.io](https://img.shields.io/crates/v/mqrstt.svg)](https://crates.io/crates/mqrstt)
[![Docs](https://docs.rs/mqrstt/badge.svg)](https://docs.rs/mqrstt)
[![dependency status](https://deps.rs/repo/github/GunnarMorrigan/mqrstt/status.svg)](https://deps.rs/repo/github/GunnarMorrigan/mqrstt)
[![codecov](https://codecov.io/github/GunnarMorrigan/mqrstt/branch/main/graph/badge.svg)](https://app.codecov.io/gh/GunnarMorrigan/mqrstt)

`MQRSTT` is an MQTTv5 client that provides sync and async (smol and tokio) implementation.

Because this crate aims to be runtime agnostic the user is required to provide their own data stream.
For an async approach the stream has to implement the smol or tokio [`AsyncReadExt`] and [`AsyncWriteExt`] traits.
For a sync approach the stream has to implement the [`std::io::Read`] and [`std::io::Write`] traits.

</div>

## Features
  - MQTT v5
  - Runtime agnostic (Smol, Tokio)
  - Sync 
  - TLS/TCP
  - Lean
  - Keep alive depends on actual communication
  
  ### To do
    - QUIC via QUINN
    - Even More testing
    - More documentation

## Examples
  ### Notes:
  - Your handler should not wait too long
  - Create a new connection when an error or disconnect is encountered
  - Handlers only get incoming packets

  ### TLS:
  TLS examples are too larger for a README due to the TLS configuration with but not limited too [tokio_rustls](https://docs.rs/tokio-rustls/latest/tokio_rustls), [async_rustls](https://docs.rs/async-rustls/latest/async_rustls) or [rustls](https://docs.rs/rustls/latest/rustls). 

### Smol example:
```rust
use mqrstt::{
    MqttClient,
    ConnectOptions,
    new_smol,
    packets::{self, Packet},
    AsyncEventHandler, NetworkStatus,
};
use async_trait::async_trait;
use bytes::Bytes;
pub struct PingPong {
    pub client: MqttClient,
}
#[async_trait]
impl AsyncEventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    async fn handle(&mut self, event: packets::Packet) -> () {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client
                            .publish(
                                p.qos,
                                p.retain,
                                p.topic.clone(),
                                Bytes::from_static(b"pong"),
                            )
                            .await
                            .unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            },
            Packet::ConnAck(_) => { println!("Connected!") },
            _ => (),
        }
    }
}
smol::block_on(async {
    let options = ConnectOptions::new("mqrsttSmolExample".to_string());
    let (mut network, client) = new_smol(options);
    let stream = smol::net::TcpStream::connect(("broker.emqx.io", 1883))
        .await
        .unwrap();
    network.connect(stream).await.unwrap();

    // This subscribe is only processed when we run the network
    client.subscribe("mqrstt").await.unwrap();

    let mut pingpong = PingPong {
        client: client.clone(),
    };
    let (n, t) = futures::join!(
        async {
            loop {
                return match network.poll(&mut pingpong).await {
                    Ok(NetworkStatus::Active) => continue,
                    otherwise => otherwise,
                };
            }
        },
        async {
            smol::Timer::after(std::time::Duration::from_secs(30)).await;
            client.disconnect().await.unwrap();
        }
    );
    assert!(n.is_ok());
});
```

### Tokio example:
```rust

use mqrstt::{
    MqttClient,
    ConnectOptions,
    new_tokio,
    packets::{self, Packet},
    AsyncEventHandler, NetworkStatus,
};
use tokio::time::Duration;
use async_trait::async_trait;
use bytes::Bytes;

pub struct PingPong {
    pub client: MqttClient,
}
#[async_trait]
impl AsyncEventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    async fn handle(&mut self, event: packets::Packet) -> () {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client
                            .publish(
                                p.qos,
                                p.retain,
                                p.topic.clone(),
                                Bytes::from_static(b"pong"),
                            )
                            .await
                            .unwrap();
                        println!("Received Ping, Send pong!");
                    }
                }
            },
            Packet::ConnAck(_) => { println!("Connected!") },
            _ => (),
        }
    }
}

#[tokio::main]
async fn main() {
    let options = ConnectOptions::new("TokioTcpPingPongExample".to_string());
    
    let (mut network, client) = new_tokio(options);
    
    let stream = tokio::net::TcpStream::connect(("broker.emqx.io", 1883))
        .await
        .unwrap();
    
    network.connect(stream).await.unwrap();
    
    client.subscribe("mqrstt").await.unwrap();
    
    let mut pingpong = PingPong {
        client: client.clone(),
    };

    let (n, _) = tokio::join!(
        async {
            loop {
                return match network.poll(&mut pingpong).await {
                    Ok(NetworkStatus::Active) => continue,
                    otherwise => otherwise,
                };
            }
        },
        async {
            tokio::time::sleep(Duration::from_secs(30)).await;
            client.disconnect().await.unwrap();
        }
    );
}

```

### Sync example:
```rust
use mqrstt::{
    MqttClient,
    ConnectOptions,
    new_sync,
    packets::{self, Packet},
    EventHandler, NetworkStatus,
};
use std::net::TcpStream;
use bytes::Bytes;

pub struct PingPong {
    pub client: MqttClient,
}

impl EventHandler for PingPong {
    // Handlers only get INCOMING packets. This can change later.
    fn handle(&mut self, event: packets::Packet) -> () {
        match event {
            Packet::Publish(p) => {
                if let Ok(payload) = String::from_utf8(p.payload.to_vec()) {
                    if payload.to_lowercase().contains("ping") {
                        self.client
                            .publish_blocking(
                                p.qos,
                                p.retain,
                                p.topic.clone(),
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

network.connect(stream).unwrap();

let mut pingpong = PingPong {
    client: client.clone(),
};
let res_join_handle = std::thread::spawn(move || 
    loop {
        match network.poll(&mut pingpong) {
            Ok(NetworkStatus::Active) => continue,
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


## License
Licensed under

* Mozilla Public License, Version 2.0, [(MPL-2.0)](https://choosealicense.com/licenses/mpl-2.0/)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, shall be licensed under MPL-2.0, without any additional terms or
conditions.
