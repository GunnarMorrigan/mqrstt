use bytes::{BytesMut, Bytes};
use tokio::io::ReadHalf;

#[derive(Debug)]
pub struct ReadStream<S> {
    stream: ReadHalf<S>,

    /// Input buffer
    const_buffer: [u8; 4096],

    /// Write buffer
    read_buffer: BytesMut,
}

impl<S> ReadStream<S> {
    pub fn new(stream: ReadHalf<S>, const_buffer: [u8; 4096], read_buffer: BytesMut) -> Self{   
        Self{
            stream,
            const_buffer,
            read_buffer,
        }
    }
}