use bytes::{Buf, BufMut, Bytes, BytesMut};
use core::slice::Iter;
use std::future::Future;
use crate::packets::error::{DeserializeError, ReadBytes, ReadError, SerializeError};

pub(crate) fn read_fixed_header_rem_len(mut buf: Iter<u8>) -> Result<(usize, usize), ReadBytes<DeserializeError>> {
    let mut integer = 0;
    let mut length = 0;

    for i in 0..4 {
        if let Some(byte) = buf.next() {
            length += 1;
            integer += (*byte as usize & 0x7f) << (7 * i);

            if (*byte & 0b1000_0000) == 0 {
                return Ok((integer, length));
            }
        } else {
            return Err(ReadBytes::InsufficientBytes(1));
        }
    }
    Err(ReadBytes::Err(DeserializeError::MalformedPacket))
}

pub(crate) trait VariableInteger: Sized {
    fn variable_integer_len(&self) -> usize;
    fn write_variable_integer(&self, buf: &mut BytesMut) -> Result<usize, SerializeError>;
    fn read_variable_integer(buf: &mut Bytes) -> Result<(Self, usize), DeserializeError>;
    fn read_async_variable_integer<S: tokio::io::AsyncReadExt + Unpin>(stream: &mut S) -> impl Future<Output = Result<(Self, usize), ReadError>>;

}



impl VariableInteger for usize {

    fn variable_integer_len(&self) -> usize {
        if *self >= 2_097_152 {
            4
        } else if *self >= 16_384 {
            3
        } else if *self >= 128 {
            2
        } else {
            1
        }
    }
    
    fn write_variable_integer(&self, buf: &mut BytesMut) -> Result<usize, SerializeError> {
        if *self > 268_435_455 {
            return Err(SerializeError::VariableIntegerOverflow(*self as usize));
        }
    
        let mut write = *self;
    
        for i in 0..4 {
            let mut byte = (write % 128) as u8;
            write /= 128;
            if write > 0 {
                byte |= 128;
            }
            buf.put_u8(byte);
            if write == 0 {
                return Ok(i+1);
            }
        }
        Err(SerializeError::VariableIntegerOverflow(*self as usize))
    }
    
    fn read_variable_integer(buf: &mut Bytes) -> Result<(Self, usize), DeserializeError> {
        let mut integer = 0;
        let mut length = 0;
    
        for i in 0..4 {
            if buf.is_empty() {
                return Err(DeserializeError::MalformedPacket);
            }
            length += 1;
            let byte = buf.get_u8();
    
            integer += (byte as usize & 0x7f) << (7 * i);
    
            if (byte & 0b1000_0000) == 0 {
                return Ok((integer, length));
            }
        }
        Err(DeserializeError::MalformedPacket)
    }
    
    fn read_async_variable_integer<S: tokio::io::AsyncReadExt + Unpin>(stream: &mut S) -> impl Future<Output = Result<(Self, usize), ReadError>> {
        async move {
            let mut integer = 0;
            let mut length = 0;
        
            for i in 0..4 {
                let byte = stream.read_u8().await?;
                length += 1;
        
                integer += (byte as usize & 0x7f) << (7 * i);
        
                if (byte & 0b1000_0000) == 0 {
                    return Ok((integer, length));
                }
            }
            Err(ReadError::DeserializeError(DeserializeError::MalformedPacket))
        }
    }
}

impl VariableInteger for u32 {
    fn variable_integer_len(&self) -> usize {
        if *self >= 2_097_152 {
            4
        } else if *self >= 16_384 {
            3
        } else if *self >= 128 {
            2
        } else {
            1
        }
    }

    fn write_variable_integer(&self, buf: &mut BytesMut) -> Result<usize, SerializeError> {
        if *self > 268_435_455 {
            return Err(SerializeError::VariableIntegerOverflow(*self as usize));
        }
    
        let mut write = *self;
    
        for i in 0..4 {
            let mut byte = (write % 128) as u8;
            write /= 128;
            if write > 0 {
                byte |= 128;
            }
            buf.put_u8(byte);
            if write == 0 {
                return Ok(i+1);
            }
        }
        Err(SerializeError::VariableIntegerOverflow(*self as usize))
    }
    
    fn read_variable_integer(buf: &mut Bytes) -> Result<(Self, usize), DeserializeError> {
        let mut integer = 0;
        let mut length = 0;
    
        for i in 0..4 {
            if buf.is_empty() {
                return Err(DeserializeError::MalformedPacket);
            }
            length += 1;
            let byte = buf.get_u8();
    
            integer += (byte as u32 & 0x7f) << (7 * i);
    
            if (byte & 0b1000_0000) == 0 {
                return Ok((integer, length));
            }
        }
        Err(DeserializeError::MalformedPacket)
    }
    
    fn read_async_variable_integer<S: tokio::io::AsyncReadExt + Unpin>(stream: &mut S) -> impl Future<Output = Result<(Self, usize), ReadError>> {
        async move {
            let mut integer = 0;
            let mut length = 0;
        
            for i in 0..4 {
                let byte = stream.read_u8().await?;
                length += 1;
        
                integer += (byte as u32 & 0x7f) << (7 * i);
        
                if (byte & 0b1000_0000) == 0 {
                    return Ok((integer, length));
                }
            }
            Err(ReadError::DeserializeError(DeserializeError::MalformedPacket))
        }
    }
}
