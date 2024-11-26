#![no_main]

use libfuzzer_sys::fuzz_target;

#[tokio::main(flavor = "current_thread")]
async fn test(mut data: &[u8]) {
    mqrstt::packets::Packet::async_read(&mut data).await;
}

fuzz_target!(|data: &[u8]| {
    // let mut packet = bytes::BytesMut::from(data);
    // mqrstt::packets::Packet::read_from_buffer(&mut packet);
    test(data);
});
