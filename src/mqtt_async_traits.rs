use futures::Future;
use tokio::io::AsyncReadExt;

pub trait AsyncMqttRead {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<usize>> + Send + Sync;

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<()>> + Send + Sync;

    fn take()

    // async fn read_exact_into_vec(&mut self, limit: u64) -> std::io::Result<Vec<u8>>;
}

// #[cfg(feature = "a")]
// impl<S> MqttAsyncRead for S where S: futures::AsyncReadExt + Unpin {
//     fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<usize>> {
//         self.read(buf)
//     }
// }

#[cfg(feature = "tokio")]
impl<S> AsyncMqttRead for S where S: tokio::io::AsyncReadExt + Unpin + Send + Sync {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<usize>> + Send + Sync {
        self.read(buf)
    }

    fn read_exact<'a>(&'a mut self, buf: &'a mut [u8]) -> impl Future<Output = std::io::Result<()>> + Send + Sync{
        async{
            self.read_exact(buf).await.map(|_| ())
        }
    }

    // async fn read_exact_into_vec(&mut self, limit: u64) -> std::io::Result<Vec<u8>> {
    //     let mut buffer = Vec::with_capacity(limit as usize);

    //     self.take(limit).read_to_end(&mut buffer).await?;

    //     Ok(buffer)
    // }
}


pub trait MqttAsyncWrite {
    async fn flush(&mut self) -> Result<(), std::io::Error>;

    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<usize>;

    async fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<()>;
}

#[cfg(feature = "a")]
impl<S> MqttAsyncWrite for S where S: futures::AsyncWriteExt + Unpin{
    async fn flush(&mut self) -> Result<(), std::io::Error> {
        self.flush().await
    }

    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<usize> {
        self.write(buf).await
    }

    async fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<()> {
        self.write_all(buf).await
    }
}

#[cfg(feature = "tokio")]
impl<S> MqttAsyncWrite for S where S: tokio::io::AsyncWriteExt + Unpin{
    async fn flush(&mut self) -> Result<(), std::io::Error> {
        self.flush().await
    }

    async fn write<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<usize> {
        self.write(buf).await
    }

    async fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> std::io::Result<()> {
        self.write_all(buf).await
    }
}