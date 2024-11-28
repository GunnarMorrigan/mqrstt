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
