mod last_will_properties;
pub use last_will_properties::LastWillProperties;

mod connect_flags;
pub use connect_flags::ConnectFlags;

mod connect_properties;
pub use connect_properties::ConnectProperties;

mod last_will;
pub use last_will::LastWill;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::packets::error::ReadError;

use super::{
    error::{DeserializeError, SerializeError}, mqtt_trait::{MqttAsyncRead, MqttRead, MqttWrite, PacketAsyncRead, PacketRead, PacketWrite}, protocol_version::ProtocolVersion, VariableInteger, WireLength
};

/// Connect packet send by the client to the server to initialize a connection.
/// 
/// Variable Header
/// - Protocol Name and Version: Identifies the MQTT protocol and version.
/// - Connect Flags: Options like clean start, will flag, will QoS, will retain, password flag, and username flag.
/// - Keep Alive Interval: Maximum time interval between messages.
/// - Properties: Optional settings such as session expiry interval, receive maximum, maximum packet size, and topic alias maximum.
/// 
/// Payload
/// - Client Identifier: Unique ID for the client.
/// - Will Message: Optional message sent if the client disconnects unexpectedly.
/// - Username and Password: Optional credentials for authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connect {
    pub protocol_version: ProtocolVersion,

    /// 3.1.2.4 Clean Start Flag
    pub clean_start: bool,
    /// 3.1.2.5 Will Flag through option
    pub last_will: Option<LastWill>,

    /// 3.1.2.8 User Name Flag
    pub username: Option<Box<str>>,
    /// 3.1.2.9 Password Flag
    pub password: Option<Box<str>>,
    /// 3.1.2.10 Keep Alive
    pub keep_alive: u16,
    /// 3.1.2.11 CONNECT Properties
    pub connect_properties: ConnectProperties,

    /// 3.1.3.1 Client Identifier (ClientID)
    pub client_id: Box<str>,
}

impl Default for Connect {
    fn default() -> Self {
        Self {
            protocol_version: ProtocolVersion::V5,
            clean_start: true,
            last_will: None,
            username: None,
            password: None,
            keep_alive: 60,
            connect_properties: ConnectProperties::default(),
            client_id: "MQRSTT".into(),
        }
    }
}

impl PacketRead for Connect {
    fn read(_: u8, _: usize, mut buf: Bytes) -> Result<Self, DeserializeError> {
        if String::read(&mut buf)? != "MQTT" {
            return Err(DeserializeError::MalformedPacketWithInfo("Protocol not MQTT".to_string()));
        }

        let protocol_version = ProtocolVersion::read(&mut buf)?;

        let connect_flags = ConnectFlags::read(&mut buf)?;

        let clean_start = connect_flags.clean_start;
        let keep_alive = buf.get_u16();

        let connect_properties = ConnectProperties::read(&mut buf)?;

        let client_id = Box::<str>::read(&mut buf)?;
        let mut last_will = None;
        if connect_flags.will_flag {
            let retain = connect_flags.will_retain;

            last_will = Some(LastWill::read(connect_flags.will_qos, retain, &mut buf)?);
        }

        let username = if connect_flags.username { Some(Box::<str>::read(&mut buf)?) } else { None };
        let password = if connect_flags.password { Some(Box::<str>::read(&mut buf)?) } else { None };

        let connect = Connect {
            protocol_version,
            clean_start,
            last_will,
            username,
            password,
            keep_alive,
            connect_properties,
            client_id,
        };

        Ok(connect)
    }
}

impl<S> PacketAsyncRead<S> for Connect where S: tokio::io::AsyncReadExt + Unpin {
    async fn async_read(_: u8, _: usize, stream: &mut S) -> Result<(Self, usize), super::error::ReadError> {
        let mut total_read_bytes = 0;
        let expected_protocol = [0x00, 0x04, b'M', b'Q', b'T', b'T'];
        let mut protocol = [0u8; 6];
        stream.read_exact(&mut protocol).await?;

        if protocol != expected_protocol {
            return Err(ReadError::DeserializeError(DeserializeError::MalformedPacketWithInfo(format!("Protocol not MQTT: {:?}", protocol))));
        }
        let (protocol_version, _) = ProtocolVersion::async_read(stream).await?;
        let (connect_flags, _) = ConnectFlags::async_read(stream).await?;
        // Add "MQTT", protocol version and connect flags read bytes
        total_read_bytes += 6 + 1 + 1;

        let clean_start = connect_flags.clean_start;
        let keep_alive = stream.read_u16().await?;
        // Add keep alive read bytes
        total_read_bytes += 2;
        
        let (connect_properties, prop_read_bytes) = ConnectProperties::async_read(stream).await?;
        let (client_id, client_read_bytes)  = Box::<str>::async_read(stream).await?;
        total_read_bytes += prop_read_bytes + client_read_bytes;

        let last_will = if connect_flags.will_flag {
            let retain = connect_flags.will_retain;
            let (last_will, last_will_read_bytes) = LastWill::async_read(connect_flags.will_qos, retain, stream).await?;
            total_read_bytes += last_will_read_bytes;
            Some(last_will)
        } else {
            None
        };

        let (username, username_read_bytes) = if connect_flags.username { 
            let (username, username_read_bytes) = Box::<str>::async_read(stream).await?;
            (Some(username), username_read_bytes)
            } else { (None, 0) };
        let (password, password_read_bytes) = if connect_flags.password { 
            let (password, password_read_bytes) = Box::<str>::async_read(stream).await?;
            (Some(password), password_read_bytes)
            } else { (None, 0) };

        total_read_bytes += username_read_bytes + password_read_bytes;

        let connect = Connect {
            protocol_version,
            clean_start,
            last_will,
            username,
            password,
            keep_alive,
            connect_properties,
            client_id,
        };
        Ok((connect, total_read_bytes))
    }
}

impl PacketWrite for Connect {
    fn write(&self, buf: &mut BytesMut) -> Result<(), SerializeError> {
        "MQTT".write(buf)?;

        self.protocol_version.write(buf)?;

        let mut connect_flags = ConnectFlags {
            clean_start: self.clean_start,
            ..Default::default()
        };

        if let Some(last_will) = &self.last_will {
            connect_flags.will_flag = true;
            connect_flags.will_retain = last_will.retain;
            connect_flags.will_qos = last_will.qos;
        }
        connect_flags.username = self.username.is_some();
        connect_flags.password = self.password.is_some();

        connect_flags.write(buf)?;

        buf.put_u16(self.keep_alive);

        self.connect_properties.write(buf)?;

        self.client_id.write(buf)?;

        if let Some(last_will) = &self.last_will {
            last_will.write(buf)?;
        }
        if let Some(username) = &self.username {
            username.write(buf)?;
        }
        if let Some(password) = &self.password {
            password.write(buf)?;
        }
        Ok(())
    }
}

impl WireLength for Connect {
    fn wire_len(&self) -> usize {
        let mut len = "MQTT".wire_len() + 1 + 1 + 2; // protocol version, connect_flags and keep alive

        len += self.connect_properties.wire_len().variable_integer_len();
        len += self.connect_properties.wire_len();

        if let Some(last_will) = &self.last_will {
            len += last_will.wire_len();
        }
        if let Some(username) = &self.username {
            len += username.wire_len()
        }
        if let Some(password) = &self.password {
            len += password.wire_len()
        }

        len += self.client_id.wire_len();

        len
    }
}

#[cfg(test)]
mod tests {
    use crate::packets::{
        mqtt_trait::{MqttWrite, PacketAsyncRead, PacketRead, PacketWrite},
        QoS,
    };

    use super::{Connect, ConnectFlags, LastWill};

    #[test]
    fn read_connect() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            // 0x10,
            // 39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x05,
            0b1100_1110, // Connect Flags, username, password, will retain=false, will qos=1, last_will, clean_start
            0x00,        // Keep alive = 10 sec
            0x0a,
            0x00, // Length of Connect properties
            0x00, // client_id length
            0x04,
            b't', // client_id
            b'e',
            b's',
            b't',
            0x00, // Will properties length
            0x00, // length topic
            0x02,
            b'/', // Will topic = '/a'
            b'a',
            0x00, // Will payload length
            0x0B,
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
            0x00, // length username
            0x04,
            b'u', // username = 'user'
            b's',
            b'e',
            b'r',
            0x00, // length password
            0x04,
            b'p', // Password = 'pass'
            b'a',
            b's',
            b's',
            0xAB, // extra packets in the stream
            0xCD,
            0xEF,
        ];

        buf.extend_from_slice(packet);
        let c = Connect::read(0, 0, buf.into()).unwrap();

        dbg!(c);
    }

    #[test]
    fn read_and_write_connect() {
        let mut buf = bytes::BytesMut::new();
        let packet = &[
            // 0x10,
            // 39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x05,        // variable header
            0b1100_1110, // variable header. +username, +password, -will retain, will qos=1, +last_will, +clean_session
            0x00,        // Keep alive = 10 sec
            0x0a,
            0x00, // Length of Connect properties
            0x00, // client_id length
            0x04,
            b't', // client_id
            b'e',
            b's',
            b't',
            0x00, // Will properties length
            0x00, // length topic
            0x02,
            b'/', // Will topic = '/a'
            b'a',
            0x00, // Will payload length
            0x0B,
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
            0x00, // length username
            0x04,
            b'u', // username
            b's',
            b'e',
            b'r',
            0x00, // length password
            0x04,
            b'p', // payload. password = 'pass'
            b'a',
            b's',
            b's',
        ];

        buf.extend_from_slice(packet);
        let c = Connect::read(0, 0, buf.into()).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf).unwrap();

        assert_eq!(packet.to_vec(), write_buf.to_vec());

        dbg!(c);
    }

    #[tokio::test]
    async fn read_async_and_write_connect() {
        let packet = &[
            // 0x10,
            // 39, // packet type, flags and remaining len
            0x00,
            0x04,
            b'M',
            b'Q',
            b'T',
            b'T',
            0x05,        // variable header
            0b1100_1110, // variable header. +username, +password, -will retain, will qos=1, +last_will, +clean_session
            0x00,        // Keep alive = 10 sec
            0x0a,
            0x00, // Length of Connect properties
            0x00, // client_id length
            0x04,
            b't', // client_id
            b'e',
            b's',
            b't',
            0x00, // Will properties length
            0x00, // length topic
            0x02,
            b'/', // Will topic = '/a'
            b'a',
            0x00, // Will payload length
            0x0B,
            b'h', // Will payload = 'hello world'
            b'e',
            b'l',
            b'l',
            b'o',
            b' ',
            b'w',
            b'o',
            b'r',
            b'l',
            b'd',
            0x00, // length username
            0x04,
            b'u', // username
            b's',
            b'e',
            b'r',
            0x00, // length password
            0x04,
            b'p', // password = 'pass'
            b'a',
            b's',
            b's',
        ];

        let (c, read_bytes) = Connect::async_read(0, 0, &mut packet.as_slice()).await.unwrap();
        assert_eq!(packet.len(), read_bytes);

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf).unwrap();

        assert_eq!(packet.to_vec(), write_buf.to_vec());
    }

    #[test]
    fn parsing_last_will() {
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, b'/', b'a', // Will topic = '/a'
            0x00, 0x0B, // Will payload length
            b'h', b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd', // Will payload = 'hello world'
        ];
        let mut buf = bytes::Bytes::from_static(last_will);
        assert!(LastWill::read(QoS::AtLeastOnce, false, &mut buf).is_ok());
    }

    #[test]
    fn read_and_write_connect2() {
        let _packet = [
            0x10, 0x1d, 0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x80, 0x00, 0x3c, 0x05, 0x11, 0xff, 0xff, 0xff, 0xff, 0x00, 0x05, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x00, 0x04, 0x54, 0x65, 0x73, 0x74,
        ];

        let data = [
            0x00, 0x04, 0x4d, 0x51, 0x54, 0x54, 0x05, 0x80, 0x00, 0x3c, 0x05, 0x11, 0xff, 0xff, 0xff, 0xff, 0x00, 0x05, 0x39, 0x2e, 0x30, 0x2e, 0x31, 0x00, 0x04, 0x54, 0x65, 0x73, 0x74,
        ];

        let mut buf = bytes::BytesMut::new();
        buf.extend_from_slice(&data);

        let c = Connect::read(0, 0, buf.into()).unwrap();

        dbg!(c.clone());

        let mut write_buf = bytes::BytesMut::new();
        c.write(&mut write_buf).unwrap();

        assert_eq!(data.to_vec(), write_buf.to_vec());
    }

    #[test]
    fn parsing_and_writing_last_will() {
        let last_will = &[
            0x00, // Will properties length
            0x00, // length topic
            0x02, b'/', // Will topic = '/a'
            b'a', 0x00, // Will payload length
            0x0B, b'h', // Will payload = 'hello world'
            b'e', b'l', b'l', b'o', b' ', b'w', b'o', b'r', b'l', b'd',
        ];
        let mut buf = bytes::Bytes::from_static(last_will);

        let lw = LastWill::read(QoS::AtLeastOnce, false, &mut buf).unwrap();

        let mut write_buf = bytes::BytesMut::new();
        lw.write(&mut write_buf).unwrap();

        assert_eq!(last_will.to_vec(), write_buf.to_vec());
    }

    #[test]
    fn connect_flag() {
        let byte = 0b1100_1110;
        let flags = ConnectFlags::from_u8(byte).unwrap();
        assert_eq!(byte, flags.into_u8().unwrap());
    }
}
