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
