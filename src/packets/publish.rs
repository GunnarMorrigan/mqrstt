use super::{QoS};


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publish{
    pub dup: bool,
    pub qos: QoS,
    pub retain: bool,
    pub topic: String,
    pub payload: Vec<u8>,
    // 2.2.1
    pub packet_identifier: Option<u16>,

    pub publish_properties: PublishProperties,

}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishProperties{
    payload_format_indicator: Option<u8>,
    message_expiry_interval: Option<u32>,
    content_type: Option<String>,
    response_topic: Option<String>,
    correlation_data: Vec<u8>,
    subscription_identifier: Vec<usize>,
    topic_alias: Option<u16>,
    user_property: Option<(String, String)>,
}