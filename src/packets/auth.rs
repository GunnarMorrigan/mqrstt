use bytes::Bytes;

use super::{
    error::DeserializeError,
    mqtt_traits::{MqttRead, MqttWrite, VariableHeaderRead, VariableHeaderWrite, WireLength},
    read_variable_integer,
    reason_codes::AuthReasonCode,
    write_variable_integer, PacketType, PropertyType,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Auth {
    pub reason_code: AuthReasonCode,
    pub properties: AuthProperties,
}

impl VariableHeaderRead for Auth {
    fn read(_: u8, _: usize, mut buf: Bytes) -> Result<Self, super::error::DeserializeError> {
        let reason_code = AuthReasonCode::read(&mut buf)?;
        let properties = AuthProperties::read(&mut buf)?;

        Ok(Self {
            reason_code,
            properties,
        })
    }
}

impl VariableHeaderWrite for Auth {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        self.reason_code.write(buf)?;
        self.properties.write(buf)?;
        Ok(())
    }
}

impl WireLength for Auth {
    fn wire_len(&self) -> usize {
        1 + self.properties.wire_len()
    }
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct AuthProperties {
    /// 3.15.2.2.2 Authentication Method
    /// 21 (0x15) Byte, Identifier of the Authentication Method.
    pub authentication_method: Option<String>,

    /// 3.15.2.2.3 Authentication Data
    /// 22 (0x16) Byte, Identifier of the Authentication Data
    pub authentication_data: Bytes,

    /// 3.15.2.2.4 Reason String
    /// 31 (0x1F) Byte, Identifier of the Reason String
    pub reason_string: Option<String>,

    /// 3.15.2.2.5 User Property
    /// 38 (0x26) Byte, Identifier of the User Property.
    pub user_properties: Vec<(String, String)>,
}

impl MqttRead for AuthProperties {
    fn read(buf: &mut Bytes) -> Result<Self, super::error::DeserializeError> {
        let (len, _) = read_variable_integer(buf)?;

        let mut properties = AuthProperties::default();

        if len == 0 {
            return Ok(properties);
        } else if buf.len() < len {
            return Err(DeserializeError::MalformedPacket);
        }

        let mut property_data = buf.split_to(len);

        loop {
            match PropertyType::read(&mut property_data)? {
                PropertyType::ReasonString => {
                    if properties.reason_string.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::SessionExpiryInterval,
                        ));
                    }
                    properties.reason_string = Some(String::read(&mut property_data)?);
                }
                PropertyType::UserProperty => properties.user_properties.push((
                    String::read(&mut property_data)?,
                    String::read(&mut property_data)?,
                )),
                PropertyType::AuthenticationMethod => {
                    if properties.authentication_method.is_some() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::AuthenticationMethod,
                        ));
                    }
                    properties.authentication_method = Some(String::read(&mut property_data)?);
                }
                PropertyType::AuthenticationData => {
                    if properties.authentication_data.is_empty() {
                        return Err(DeserializeError::DuplicateProperty(
                            PropertyType::AuthenticationData,
                        ));
                    }
                    properties.authentication_data = Bytes::read(&mut property_data)?;
                }
                e => return Err(DeserializeError::UnexpectedProperty(e, PacketType::Auth)),
            }

            if property_data.is_empty() {
                break;
            }
        }

        Ok(properties)
    }
}

impl MqttWrite for AuthProperties {
    fn write(&self, buf: &mut bytes::BytesMut) -> Result<(), super::error::SerializeError> {
        write_variable_integer(buf, self.wire_len())?;

        if let Some(authentication_method) = &self.authentication_method {
            PropertyType::AuthenticationMethod.write(buf)?;
            authentication_method.write(buf)?;
        }
        if !self.authentication_data.is_empty() && self.authentication_method.is_some() {
            PropertyType::AuthenticationData.write(buf)?;
            self.authentication_data.write(buf)?;
        }
        if let Some(reason_string) = &self.reason_string {
            PropertyType::ReasonString.write(buf)?;
            reason_string.write(buf)?;
        }
        for (key, value) in &self.user_properties {
            PropertyType::UserProperty.write(buf)?;
            key.write(buf)?;
            value.write(buf)?;
        }

        Ok(())
    }
}

impl WireLength for AuthProperties {
    fn wire_len(&self) -> usize {
        todo!()
    }
}
