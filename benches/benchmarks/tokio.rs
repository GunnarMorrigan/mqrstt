use std::{
    hint::black_box,
    io::{Cursor, Write},
    time::Duration, net::SocketAddr, sync::Arc,
};

use bytes::BytesMut;
use criterion::{criterion_group, Criterion};
use mqrstt::{NetworkBuilder, ConnectOptions, NetworkStatus};
use tokio::net::TcpStream;

use crate::benchmarks::test_handlers::{PingPong, SimpleDelay};

use super::fill_stuff;

struct ReadWriteTester<'a> {
    read: Cursor<&'a [u8]>,
    write: Vec<u8>,
}

impl<'a> ReadWriteTester<'a> {
    pub fn new(read: &'a [u8]) -> Self {
        Self {
            read: Cursor::new(read),
            write: Vec::new(),
        }
    }
}

impl<'a> tokio::io::AsyncRead for ReadWriteTester<'a> {
    fn poll_read(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>, buf: &mut tokio::io::ReadBuf<'_>) -> std::task::Poll<std::io::Result<()>> {
        tokio::io::AsyncRead::poll_read(std::pin::Pin::new(&mut self.read), cx, buf)
    }
}

impl<'a> tokio::io::AsyncWrite for ReadWriteTester<'a> {
    fn poll_write(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>, _buf: &[u8]) -> std::task::Poll<Result<usize, std::io::Error>> {
        todo!()
    }

    fn poll_flush(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }

    fn poll_shutdown(self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), std::io::Error>> {
        todo!()
    }
}

fn tokio_setup() -> (TcpStream, std::net::TcpStream, SocketAddr) {
    let mut buffer = BytesMut::new();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let tcp_stream = std::net::TcpStream::connect(addr).unwrap();

    let (mut server, _addr) = listener.accept().unwrap();

    fill_stuff(&mut buffer, 100, 5_000_000);

    server.write_all(&buffer.to_vec()).unwrap();

    let tcp_stream = tokio::net::TcpStream::from_std(tcp_stream).unwrap();
    (tcp_stream, server, _addr)
}

fn tokio_concurrent_benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tokio concurrent read, write and handling");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(120));

    group.bench_function("tokio_bench_concurrent_read_write_and_handling_NOP", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_concurrent_network();

                let _server_box = black_box(client);
                
                network.connect(tcp_stream, &mut ()).await.unwrap();
                let (read, write) = network.split(()).unwrap();
                
                let _network_box = black_box(network);
                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());

                let (read_res, write_res) = tokio::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert_eq!(read_res, NetworkStatus::IncomingDisconnect);
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);
            },
        )
    });
    group.bench_function("tokio_bench_concurrent_read_write_and_handling_PingPong", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_concurrent_network();

                let mut pingpong = Arc::new(PingPong::new(client.clone()));
                
                network.connect(tcp_stream, &mut pingpong).await.unwrap();
                let (read, write) = network.split(pingpong.clone()).unwrap();
                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());
                
                let (read_res, write_res) = futures::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert_eq!(read_res, NetworkStatus::IncomingDisconnect);
                assert_eq!(102, pingpong.number.load(std::sync::atomic::Ordering::SeqCst));
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);
                
                let _server_box = black_box(client.clone());
                let _server_box = black_box(server);
                let _addr_box = black_box(addr);
                let _network_box = black_box(network);
            },
        )
    });
    group.bench_function("tokio_bench_concurrent_read_write_and_handling_100ms_Delay", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_concurrent_network();

                let _server_box = black_box(client);

                let mut handler = Arc::new(SimpleDelay::new(Duration::from_millis(100)));

                network.connect(tcp_stream, &mut handler).await.unwrap();
                let (read, write) = network.split(handler).unwrap();

                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());
                
                let (read_res, write_res) = tokio::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                assert_eq!(read_res.unwrap(), NetworkStatus::IncomingDisconnect);
                
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);

                let _network_box = black_box(network);
            },
        )
    });

    group.bench_function("tokio_bench_concurrent_read_write", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let _server_box = black_box(client);

                network.connect(tcp_stream, &mut ()).await.unwrap();

                let (read, write) = network.split(()).unwrap();
                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());

                let (read_res, write_res) = tokio::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                assert_eq!(read_res.unwrap(), NetworkStatus::IncomingDisconnect);
                
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);
            }
        )
    });
    group.bench_function("tokio_bench_concurrent_read_write_PingPong", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let mut pingpong = PingPong::new(client.clone());

                let num_packets_received = pingpong.number.clone();
                
                network.connect(tcp_stream, &mut pingpong).await.unwrap();
                let (read, write) = network.split(pingpong).unwrap();
                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());
                
                let (read_res, write_res) = futures::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                assert_eq!(read_res.unwrap(), NetworkStatus::IncomingDisconnect);
                assert_eq!(102, num_packets_received.load(std::sync::atomic::Ordering::SeqCst));
                
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);
                
                let _server_box = black_box(client.clone());
                let _server_box = black_box(server);
                let _addr_box = black_box(addr);
                let _network_box = black_box(network);
            },
        )
    });
    group.bench_function("tokio_bench_concurrent_read_write_100ms_Delay", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let _server_box = black_box(client);

                let mut handler = SimpleDelay::new(Duration::from_millis(100));

                network.connect(tcp_stream, &mut handler).await.unwrap();
                let (read, write) = network.split(handler).unwrap();

                
                let read_handle = tokio::task::spawn(read.run());
                let write_handle = tokio::task::spawn(write.run());
                
                let (read_res, write_res) = futures::join!(read_handle, write_handle);
                assert!(read_res.is_ok());
                let read_res = read_res.unwrap();
                assert!(read_res.is_ok());
                assert_eq!(read_res.unwrap(), NetworkStatus::IncomingDisconnect);
                
                assert_eq!(write_res.unwrap().unwrap(), NetworkStatus::ShutdownSignal);

                let _network_box = black_box(network);
            },
        )
    });
}


fn tokio_synchronous_benchmarks(c: &mut Criterion){
    let mut group = c.benchmark_group("Tokio sequential");
    group.sample_size(30);
    group.measurement_time(Duration::from_secs(120));

    group.bench_function("tokio_bench_sync_read_write", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let _server_box = black_box(client);

                network.connect(tcp_stream, &mut ()).await.unwrap();

                let network_res = network.run(&mut ()).await;

                assert!(network_res.is_ok());
                let network_res = network_res.unwrap();
                assert_eq!(network_res, NetworkStatus::IncomingDisconnect);
            }
        )
    });
    group.bench_function("tokio_bench_sync_read_write_PingPong", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let mut pingpong = PingPong::new(client.clone());

                let _server_box = black_box(client);

                network.connect(tcp_stream, &mut pingpong).await.unwrap();

                let network_res = network.run(&mut pingpong).await;

                assert!(network_res.is_ok());
                let network_res = network_res.unwrap();
                assert_eq!(network_res, NetworkStatus::IncomingDisconnect);
            }
        )
    });
    group.bench_function("tokio_bench_sync_read_write_100ms_Delay", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.to_async(runtime).iter_with_setup(
            tokio_setup,
            |(tcp_stream, server, addr)| async move {
                let _server_box = black_box(server);
                let _addr = black_box(addr);

                let options = ConnectOptions::new("test");
                let (mut network, client) = NetworkBuilder::new_from_options(options).tokio_sequential_network();

                let mut handler = SimpleDelay::new(Duration::from_millis(100));

                let _server_box = black_box(client);

                network.connect(tcp_stream, &mut handler).await.unwrap();

                let network_res = network.run(&mut handler).await;

                assert!(network_res.is_ok());
                let network_res = network_res.unwrap();
                assert_eq!(network_res, NetworkStatus::IncomingDisconnect);
            }
        )
    });
}

criterion_group!(tokio_concurrent, tokio_concurrent_benchmarks);
criterion_group!(tokio_synchronous, tokio_synchronous_benchmarks);
