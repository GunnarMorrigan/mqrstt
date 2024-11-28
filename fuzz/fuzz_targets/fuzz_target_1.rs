#![no_main]

use libfuzzer_sys::fuzz_target;

#[tokio::main(flavor = "current_thread")]
async fn test(mut data: &[u8]) {
    let _ = mqrstt::packets::Packet::async_read(&mut data).await;
}

#[cfg(target_os = "linux")]
fuzz_target!(|data: &[u8]| {
    test(data);
});
