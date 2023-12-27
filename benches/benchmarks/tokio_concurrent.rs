use std::{io::{Write, Cursor}, hint::black_box, time::Duration};

use bytes::BytesMut;
use criterion::{criterion_group, BatchSize, Criterion};
use mqrstt::{new_tokio, ConnectOptions};

use super::fill_stuff;

struct ReadWriteTester<'a>{
    read: Cursor<&'a [u8]>,
    write: Vec<u8>
}

impl<'a> ReadWriteTester<'a> {
    pub fn new(read: &'a [u8]) -> Self {
        Self{
            read: Cursor::new(read),
            write: Vec::new(),
        }
    }
}

impl<'a> tokio::io::AsyncRead for ReadWriteTester<'a> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        tokio::io::AsyncRead::poll_read(std::pin::Pin::new(&mut self.read), cx, buf)
    }
}

impl<'a> tokio::io::AsyncWrite for ReadWriteTester<'a> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}


fn tokio_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tokio concurrent throughput test");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));

    group.bench_function("tokio_bench_concurrent_read_write", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime)
            .iter_with_setup(
            || {
                let mut buffer = BytesMut::new();
                
                // :0 tells the OS to pick an open port.
                let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
                let addr = listener.local_addr().unwrap();
                
                let tcp_stream = std::net::TcpStream::connect(addr).unwrap();
                                
                let (mut server, _addr) = listener.accept().unwrap();

                fill_stuff(&mut buffer, 100, 5_000_000);

                server.write_all(&buffer.to_vec()).unwrap();

                let tcp_stream = tokio::net::TcpStream::from_std(tcp_stream).unwrap();
                (tcp_stream, server, _addr)
            },
            |(tcp_stream, server, addr)| async move { 

                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test", true);
                let (mut network, _) = new_tokio(options);

                network.connect(tcp_stream, ()).await.unwrap();

                network.run().await.unwrap();
            })
    });
}

criterion_group!(tokio, tokio_concurrent);