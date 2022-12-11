// use std::{time::Instant, sync::Arc, future::{self}};

// use async_channel::Sender;
// use futures::future::Pending;

// use crate::{connections::Outgoing, error::{ConnectionError, MqttError}};

// pub struct KeepAlive{
//     last_communication: Arc<Instant>
// }

// impl KeepAlive{
//     pub fn new(sender: Sender<Outgoing>, keep_alive_s: u64) -> Self{
//         Self { 
//             last_communication: Arc::new(Instant::now())
//         }
//     }   
//     pub fn new_empty() -> Self{
//         Self { 
//             last_communication: Arc::new(Instant::now())
//         }
//     }

//     pub async fn keep_alive_process() -> Result<(), MqttError>{
//         let future = future::pending();
//         let () = future.await;
//         todo!()
//         // loop {
//         //     tokio::time::sleep(tokio::time::Duration::from_secs(sleep_time_s)).await;
//         //     if false {
//         //         // sleep_time_s = ;
//         //         continue;
//         //     }
//         //     sleep_time_s = keep_alive_s;
//         //     sender.send(Packet::PingReq);
//         // }
//     }
// }