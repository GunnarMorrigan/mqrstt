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

#[tokio::main]
async fn main() {
    let hostname = "broker.emqx.io:1883";

    let mut handler = Handler { byte_count: 0 };

    let stream = tokio::net::TcpStream::connect(hostname).await.unwrap();
    let stream = tokio::io::BufStream::new(stream);
    let (mut network, client) = mqrstt::NetworkBuilder::new_from_client_id("TestClientABCDEFG").tokio_network();

    network.connect(stream, &mut handler).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    client.subscribe("testtopic/#").await.unwrap();

    tokio::spawn(async move {
        network.run(&mut handler).await.unwrap();

        dbg!(handler.byte_count);
    });

    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    client.disconnect().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
