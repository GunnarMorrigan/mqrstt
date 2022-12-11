#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use client::AsyncClient;
use connect_options::ConnectOptions;
use connections::{tcp::Tcp};
use event_handler::{EventHandlerTask, EventHandler};
use network::MqttNetwork;
use packets::packets::Packet;

mod connections;
mod packets;
mod state;
mod error;
mod available_packet_ids;
pub mod connect_options;
mod event_handler;
pub mod util;
pub mod client;
mod network;

pub fn create_new_tcp<H>(options: ConnectOptions, custom_handler: H) -> (MqttNetwork<Tcp>, EventHandlerTask<H>, AsyncClient)
    where H: EventHandler + Send + Sized + 'static{
    
    let receive_maximum = options.receive_maximum();
    let (network, network_sender, network_receiver, last_comm) = MqttNetwork::<Tcp>::new(options);
    
    let (handler, handler_sender, packet_ids) = EventHandlerTask::new(
        receive_maximum,
        network_receiver,
        network_sender.clone(),
        custom_handler,
        last_comm,
    );
    let client = AsyncClient::new(packet_ids, handler_sender, network_sender);

    (network, handler, client)
}


pub struct Hello{}

impl EventHandler for Hello{

    fn handle<'a> (&mut self, event: &'a Packet) -> impl futures::Future<Output = ()> + Send + 'a{
        async move {
            tracing::warn!("Received event {:?}", event);
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use tokio::{join, net::TcpStream};
    use tracing::{Level, debug};
    use tracing_subscriber::FmtSubscriber;

    use crate::{connect_options::ConnectOptions, create_new_tcp, Hello, packets::{packets::Packet, publish::{Publish, PublishProperties}, QoS}, error::ClientError};

    #[tokio::test(flavor = "multi_thread")]
    async fn create(){

        let filter = tracing_subscriber::filter::EnvFilter::new("none,mqrstt=trace");

        let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info, warn, etc.)
        // will be written to stdout.
        .with_env_filter(filter)
        .with_max_level(Level::TRACE)
        .with_line_number(true)
        // completes the builder.
        .finish();

        tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

        let opt = ConnectOptions::new("broker.emqx.io".to_string(), 1883, "test123123".to_string());
        // let opt = ConnectOptions::new("azurewe1576.azureexternal.dnvgl.com".to_string(), 1883, "test123123".to_string());

        let (mut network, mut handler, client) = create_new_tcp(opt, Hello{});
    
        let network = tokio::task::spawn(async move{
            dbg!(network.run().await)
        });
        let handler = tokio::task::spawn(async move{
            loop{
                match handler.handle().await {
                    Ok(_) => (),
                    a => {
                        return dbg!(a);
                    },
                }
            }
        });
        let sender = tokio::task::spawn(async move{
            client.subscribe("test/123").await?;
            // client.publish(QoS::ExactlyOnce, false, "Lol".to_string(), "123456789").await?;
            debug!("Published message");
            // loop{
            //     let p = Publish::new(QoS::AtMostOnce, false, "test".to_string(), None, PublishProperties::default(), Bytes::from_static(b"lmfao bru wtf"));
            //     tracing::trace!("Sending a packet"); 
            //     client.publish(QoS::ExactlyOnce, false, "Lol".to_string(), "123456789").await?;
            //     tokio::time::sleep(tokio::time::Duration::new(10, 0)).await;
            // }
            loop{
                ()
            }
            let ret: Result<(), ClientError> = Ok(());
            ret
        });

        let a = dbg!(join!(network, handler, sender));
        println!("Hello {:?}", a);
    }


    #[tokio::test]
    async fn media(){
        println!("started");
        let connection = TcpStream::connect(("broker.emqx.io", 1883)).await;
        println!("Done");
    }
}