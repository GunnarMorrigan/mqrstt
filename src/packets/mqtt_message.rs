
pub struct Message{
    qos: QoS,
    retained: bool,
    dup: bool,
    payload: Bytes,
    message_id: u8,
    // MqttProperties properties,
}

pub struct MessageProperties{

}