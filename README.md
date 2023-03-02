<div align="center">

# `📟 mqrstt`

[![Crates.io](https://img.shields.io/crates/v/mqrstt.svg)](https://crates.io/crates/mqrstt)
[![Docs](https://docs.rs/mqrstt/badge.svg)](https://docs.rs/mqrstt)
[![dependency status](https://deps.rs/repo/github/GunnarMorrigan/mqrstt/status.svg)](https://deps.rs/repo/github/GunnarMorrigan/mqrstt)
[![codecov](https://codecov.io/github/GunnarMorrigan/mqrstt/branch/main/graph/badge.svg?token=YSZFYQ063Y)](https://codecov.io/github/GunnarMorrigan/mqrstt)

`mqrstt` is an MQTTv5 client implementation that allows for the smol and tokio runtimes. In the future we will also support a sync implementation.

</div>

## Features
  - MQTT v5
  - Retransmission
  - Runtime agnostic
  - Lean
  - Keep alive depends on actual communication
  
### To do
  - Enforce size of outbound messages (e.g. Publish)
  - Sync API
  - More testing
  - More documentation
  - Remove logging calls or move all to test flag

## Examples

You want to reconnect (with a new stream) after the network encountered an error or a disconnect took place!

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

## FAQ
 - Why are there no implementations for TLS connections?
   Many examples of creating TLS streams in rust exist with the crates [`async-rustls`](https://crates.io/crates/async-rustls) and [`tokio-rustls`](https://crates.io/crates/tokio-rustls). The focus of this crate is `MQTTv5` and providing a runtime free choice.  
  
- What are the advantages over [`rumqttc`](https://crates.io/crates/rumqttc)?
  - Handling of messages by user before acknowledgement.
  - Ping req depending on communication
  - No `rumqttc` packet id collision errors (It is not possible with `rumqtts`).
  - Runtime agnositc 
  - Mqtt version 5 support

 - Please ask :)

## Important notes:
 - Handlers only get incoming packets.

## Size
With the smol runtime you can create very small binaries. A simple PingPong smol TCP client can be had for 550\~KB and with TLS you are looking at 1.5\~ MB using the following flags. This makes `mqrstt` extremely usefull for embedded devices! :)
```
[profile.release]
opt-level = "z"  # Optimize for size.
lto = true
codegen-units = 1
strip = true
```

## License

Licensed under

* Mozilla Public License, Version 2.0, [(MPL-2.0)](https://choosealicense.com/licenses/mpl-2.0/)

## Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, shall be licensed under MPL-2.0, without any additional terms or
conditions.
