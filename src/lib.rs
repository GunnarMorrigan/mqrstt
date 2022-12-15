#![feature(async_fn_in_trait)]
#![feature(return_position_impl_trait_in_trait)]

use std::{sync::Arc, time::Instant};

use async_channel::Receiver;
use async_mutex::Mutex;
use client::AsyncClient;
use connect_options::ConnectOptions;
use connections::{tcp_tokio::{tcp_reader::TcpReader, tcp_writer::TcpWriter}};
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

pub fn create_new_tcp(options: ConnectOptions) -> (MqttNetwork<TcpReader, TcpWriter>, EventHandlerTask, AsyncClient, Receiver<Packet>){
    
    let receive_maximum = options.receive_maximum();

    let (network_to_handler_s, network_to_handler_r) = async_channel::bounded(100);
    let (to_network_s, to_network_r) = async_channel::bounded(100);
    let (client_to_handler_s, client_to_handler_r) = async_channel::bounded(receive_maximum as usize);

    let last_network_action = Arc::new(Mutex::new(Instant::now()));


    let network = MqttNetwork::<TcpReader, TcpWriter>::new(options, network_to_handler_s, to_network_r, last_network_action.clone());
    
    let (handler, packet_ids) = EventHandlerTask::new(
        receive_maximum,
        network_to_handler_r,
        to_network_s.clone(),
        client_to_handler_r.clone(),
        last_network_action,
    );
    let client = AsyncClient::new(packet_ids, client_to_handler_s, to_network_s);

    (network, handler, client, client_to_handler_r)
}


pub struct Hello{}

impl EventHandler for Hello{

    fn handle<'a> (&mut self, event: &'a Packet) -> impl core::future::Future<Output = ()> + Send + 'a{
        async move {
            // tracing::warn!("Received event {:?}", event);
        }
    }
}

#[cfg(test)]
mod tests {
    use async_channel::Receiver;
    use futures_concurrency::future::Join;
    use tokio::{net::TcpStream, join};
    use tracing::{Level, warn, info, debug};
    use tracing_subscriber::FmtSubscriber;

    use crate::{connect_options::ConnectOptions, create_new_tcp, Hello, error::{ClientError, MqttError}, packets::QoS};

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

        // let opt = ConnectOptions::new("broker.emqx.io".to_string(), 1883, "test123123".to_string());
        let opt = ConnectOptions::new("azurewe1576.azureexternal.dnvgl.com".to_string(), 1883, "test123123".to_string());

        let (mut mqtt_network, mut handler, client, r) = create_new_tcp(opt);
    
        let network = tokio::task::spawn(async move{
            dbg!(mqtt_network.run_with_shutdown_signal().await)
        });

        let event_handler = tokio::task::spawn(async move{
            let mut custom_handler = Hello{};
            loop{
                match handler.handle(&mut custom_handler).await{
                    Ok(_) => (),
                    a => {
                        return dbg!(a);
                    },
                }
            }
        });
        
        let sender = tokio::task::spawn(async move{

            client.publish(QoS::ExactlyOnce, false, "test/123".to_string(), "123456789").await?;

            let lol = smol::future::pending::<Result<(), ClientError>>();
            lol.await
        });

        // dbg!(network.await);
        let a = dbg!((network, event_handler, sender).join().await);
        // println!("Hello {:?}", a);
    }
}