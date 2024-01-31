use std::time::Duration;

use mqrstt::{
    new_sync, packets::{self, Packet}, sync::NetworkStatus, ConnectOptions, EventHandler, MqttClient
};

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
                        self.client.publish_blocking(p.topic.clone(), p.qos, p.retain, "pong").unwrap();
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
    let client_id = "SyncTcp_MQrsTT_Example".to_string();
    let options = ConnectOptions::new(client_id);

    let address = "broker.emqx.io";
    let port = 1883;

    let (mut network, client) = new_sync(options);

    let stream = std::net::TcpStream::connect((address, port)).unwrap();
    stream.set_nonblocking(true).unwrap();

    let mut pingpong = PingPong { client: client.clone() };

    network.connect(stream, &mut pingpong).unwrap();

    client.subscribe_blocking("mqrstt").unwrap();

    let thread = std::thread::spawn(move || {
        loop {
            match network.poll(&mut pingpong) {
                // The client is active but there is no data to be read
                Ok(NetworkStatus::ActivePending) => std::thread::sleep(Duration::from_millis(100)),
                // The client is active and there is data to be read
                Ok(NetworkStatus::ActiveReady) => continue,
                // The rest is an error
                otherwise => return otherwise,
            };
        }
    });

    std::thread::sleep(std::time::Duration::from_secs(30));
    client.disconnect_blocking().unwrap();
    
    // Unwrap possible join errors on the thread.
    let n = thread.join().unwrap();
    assert!(n.is_ok());
}
