use std::fmt::Display;

use futures_concurrency::future::{Race};


#[derive(Debug, Clone, Copy)]
pub struct Timeout(());

impl Display for Timeout{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Timeout")
    }
}

impl std::error::Error for Timeout{}

pub async fn timeout<T>(fut: impl core::future::Future<Output = T>, delay_seconds: u64 ) -> Result<T, Timeout> {
    (
        async{
            Ok(fut.await)
        },
        async{
            #[cfg(feature = "smol")]
            smol::Timer::after(std::time::Duration::from_secs(delay_seconds)).await;
            #[cfg(feature = "tokio")]
            tokio::time::sleep(tokio::time::Duration::from_secs(delay_seconds)).await;
            Err(Timeout(()))
        }
    ).race().await
}