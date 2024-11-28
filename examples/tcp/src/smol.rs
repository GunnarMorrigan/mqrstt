use mqrstt::AsyncEventHandler;

pub struct Handler {
    byte_count: u64,
}

impl AsyncEventHandler for Handler {
    async fn handle(&mut self, incoming_packet: mqrstt::packets::Packet) {
        if let mqrstt::packets::Packet::Publish(publish) = incoming_packet {
            self.byte_count += publish.payload.len() as u64;
        }
    }
}

fn main() {
    smol::block_on(async {
        let hostname = "broker.emqx.io:1883";

        let mut handler = Handler { byte_count: 0 };

        let stream = smol::net::TcpStream::connect(hostname).await.unwrap();
        let (mut network, client) = mqrstt::NetworkBuilder::new_from_client_id("TestClientABCDEFG").smol_network();

        network.connect(stream, &mut handler).await.unwrap();
        smol::Timer::after(std::time::Duration::from_secs(5)).await;

        client.subscribe("testtopic/#").await.unwrap();

        smol::spawn(async move {
            network.run(&mut handler).await.unwrap();

            dbg!(handler.byte_count);
        })
        .detach();

        smol::Timer::after(std::time::Duration::from_secs(60)).await;
        client.disconnect().await.unwrap();
        smol::Timer::after(std::time::Duration::from_secs(1)).await;
    });
}
