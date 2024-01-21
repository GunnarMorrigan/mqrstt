

use async_trait::async_trait;
use mqrstt::{MqttClient, AsyncEventHandler, packets::{self, Packet}, ConnectOptions, new_smol, smol::NetworkStatus};


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
                                p.topic.clone(),
                                p.qos,
                                p.retain,
                                "pong",
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

fn main() {
    smol::block_on(async {
        let client_id = "SmolTls_MQrsTT_Example".to_string();
        let options = ConnectOptions::new(client_id);

        let address = "broker.emqx.io";
        let port = 8883;

        let (mut network, client) = new_smol(options);

        let stream = smol::net::TcpStream::connect((address, port)).await.unwrap();

        let mut pingpong = PingPong { client: client.clone() };

        network.connect(stream, &mut pingpong).await.unwrap();

        client.subscribe("mqrstt").await.unwrap();

        let (n, _) = futures::join!(
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
}
