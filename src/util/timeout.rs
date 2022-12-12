use async_io::Async;
use futures_concurrency::future::{Race};


pub struct Timeout(());

pub async fn timeout<T>(fut: impl core::future::Future<Output = T>, delay_seconds: u64 ) -> Result<T, Timeout> {
    (
        async{
            Ok(fut.await)
        },
        async{
            #[cfg(feature = "smol")]
            async_io::Timer::after(std::time::Duration::from_secs(delay_seconds)).await;
            #[cfg(feature = "tokio")]
            tokio::time::sleep(tokio::time::Duraction::from_secs(delay_seconds)).await;
            Err(Timeout(()))
        }
    ).race().await
}