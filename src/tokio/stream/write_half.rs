use bytes::BytesMut;
use tokio::io::WriteHalf;

#[derive(Debug)]
pub struct WriteStream<S> {
    pub stream: WriteHalf<S>,

    /// Write buffer
    write_buffer: BytesMut,
}

impl<S> WriteStream<S> {
    pub fn new(stream: WriteHalf<S>, write_buffer: BytesMut) -> Self{   
        Self{
            stream,
            write_buffer,
        }
    }
}