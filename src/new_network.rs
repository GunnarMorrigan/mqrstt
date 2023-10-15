pub struct Network<T>{
    inner: T,
}

impl<T> Network<std::io::BufReader<T>> {
    pub fn new<R: std::io::Read + std::io::Write>(inner: R) -> Network<std::io::BufReader<R>>{
        Network{
            inner: std::io::BufReader::new(inner),
        }
    }

    // pub fn read<R: std::io::Read>(&mut self, reader: &R) -> Result<usize, std::io::Error> {
    //     match reader.read(&mut self.inner[self.write_pos..]) {
    //         Ok(n) => {
    //             self.write_pos + n;
    //             Ok(n)
    //         },
    //         Err(err) => Err(err),
    //     }
    // }

    // pub async fn async_read_tokio<R: tokio::io::AsyncRead + Unpin>(&mut self, reader: &mut R) -> Result<usize, std::io::Error> {
    //     match tokio::io::AsyncReadExt::read(reader, &mut self.inner[self.write_pos..]).await {
    //         Ok(n) => {
    //             self.write_pos + n;
    //             Ok(n)
    //         },
    //         Err(err) => Err(err), 
    //     }
    // }

    // pub async fn async_read_smol<R: smol::io::AsyncRead + Unpin>(&mut self, reader: &mut R) -> Result<usize, std::io::Error> {
    //     match smol::io::AsyncReadExt::read(reader, &mut self.inner[self.write_pos..]).await {
    //         Ok(n) => {
    //             self.write_pos + n;
    //             Ok(n)
    //         },
    //         Err(err) => Err(err), 
    //     }
    // }
}
