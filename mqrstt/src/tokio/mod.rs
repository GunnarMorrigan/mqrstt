mod stream;

pub(crate) mod network;

use futures::Future;
pub use network::Network;
pub use network::{NetworkReader, NetworkWriter};

use crate::error::ConnectionError;
use crate::packets::Packet;

/// This empty struct is used to indicate the handling of messages goes via a mutable handler.
/// Only a single mutable reference can exist at once. 
/// Thus this kind is not for concurrent message handling but for concurrent TCP read and write operations.
pub struct SequentialHandler;

/// This empty struct is used to indicate a (tokio) task based handling of messages.
/// Per incoming message a task is spawned to call the handler.
/// 
/// This kind of handler is used for both concurrent message handling and concurrent TCP read and write operations.
pub struct ConcurrentHandler;

pub trait HandlerExt<H>: Sized{
    /// Should call the handler in the fashion of the handler.
    /// (e.g. spawn a task if or await the handle call)
    fn call_handler(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send;
    
    /// Should call the handler and await it
    fn call_handler_await(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send;

    /// Should call the handler in the fashion of the handler.
    /// (e.g. spawn a task if or await the handle call)
    /// The reply (e.g. an ACK) to the original packet is only send when the handle call has completed
    fn call_handler_with_reply<S>(
        network: &mut NetworkReader<Self, H, S>,
        incoming_packet: Packet,
        reply_packet: Option<Packet>
    ) -> impl Future<Output = Result<(), ConnectionError>> + Send
    where 
        S: Send
    ;

}

impl<H: crate::AsyncEventHandlerMut + Send> HandlerExt<H> for SequentialHandler {
    #[inline]
    fn call_handler(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send{
        handler.handle(incoming_packet)
    }
    #[inline]
    fn call_handler_await(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send{
        handler.handle(incoming_packet)
    }
    fn call_handler_with_reply<S>(network: &mut NetworkReader<Self, H, S>, incoming_packet: Packet, reply_packet: Option<Packet>) -> impl Future<Output = Result<(), ConnectionError>> + Send 
    where
        S: Send,
    {
        async{
            network.handler.handle(incoming_packet).await;
            if let Some(reply_packet) = reply_packet {
                network.to_writer_s.send(reply_packet).await?;
            }
            Ok(())
        }
    }
}

impl<H: crate::AsyncEventHandler + Send + Sync + Clone + 'static> HandlerExt<H> for ConcurrentHandler {
    fn call_handler(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send{
        let handler_clone = handler.clone();
        tokio::spawn(async move {
            handler_clone.handle(incoming_packet).await;
        });
        std::future::ready(())
    }
    #[inline]
    fn call_handler_await(handler: &mut H, incoming_packet: Packet) -> impl Future<Output = ()> + Send{
        handler.handle(incoming_packet)
    }
    
    fn call_handler_with_reply<S>(network: &mut NetworkReader<Self, H, S>, incoming_packet: Packet, reply_packet: Option<Packet>) -> impl Future<Output = Result<(), ConnectionError>> + Send 
    where
        S: Send,
    {
        let handler_clone = network.handler.clone();
        let write_channel_clone = network.to_writer_s.clone();

        network.join_set.spawn(async move {
            handler_clone.handle(incoming_packet).await;
            if let Some(reply_packet) = reply_packet {
                write_channel_clone.send(reply_packet).await?;
            }
            Ok(())
        });

        std::future::ready(Ok(()))
    }
}