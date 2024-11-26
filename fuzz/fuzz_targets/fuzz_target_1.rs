#![no_main]

use libfuzzer_sys::fuzz_target;

#[tokio::main(flavor = "current_thread")]
async fn test(mut data: &[u8]) {
    let _ = mqrstt::packets::Packet::async_read(&mut data).await;
}

fuzz_target!(|data: &[u8]| {
    test(data);
});
