#![feature(test)]


extern crate test;
use bencher::{Bencher, benchmark_group, benchmark_main};

use futures::join;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

async fn roundtrip_raw(){
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let peer1 = tokio::spawn(async move {
        let (mut conn, _) = listener.accept().await.unwrap();
        
        conn.write_u64(1).await.unwrap();
        assert_eq!(conn.read_u64().await.unwrap(), 2);
    });

    let peer2 = tokio::spawn(async move {
        let mut conn = TcpStream::connect("127.0.0.1:8080").await.unwrap();

        assert_eq!(conn.read_u64().await.unwrap(), 1);
        conn.write_u64(2).await.unwrap();
    });

    _ = join!(peer1, peer2);
}


async fn roundtrip_mux(){
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let peer1 = tokio::spawn(async move {
        let (conn, _) = listener.accept().await.unwrap();
        let mux = tokio_smux::Mux::new(conn, tokio_smux::Config::new(1, 1, 16192 as usize));
        let mut conn = mux.accept().await.unwrap();
        
        conn.write_u64(1).await.unwrap();
        assert_eq!(conn.read_u64().await.unwrap(), 2);
    });

    let peer2 = tokio::spawn(async move {
        let conn = TcpStream::connect("127.0.0.1:8080").await.unwrap();
        let mux = tokio_smux::Mux::new(conn, tokio_smux::Config::new(1, 1, 16192 as usize));
        let mut conn = mux.connect().await.unwrap();

        assert_eq!(conn.read_u64().await.unwrap(), 1);
        conn.write_u64(2).await.unwrap();

        _ = peer1.await;
    });

    _ = peer2.await;
}

async fn transfer_mux() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let read_handle = tokio::spawn(async move {
        let (conn, _) = listener.accept().await.unwrap();
        let conn = tokio_smux::Mux::new(conn, tokio_smux::Config::new(1, 1, 16192 as usize));
        let mut stream = conn.accept().await.unwrap();
        let mut buf = [0u8; 8092];
        let mut total = 0;
        // let now = std::time::Instant::now();
        while total < 1024 * 1024 * 256 {
            let n = stream.read(&mut buf).await.unwrap();
            total += n;
        }
        // println!("read elapsed: {:?}", now.elapsed());
    });

    let write_handle = tokio::spawn(async move {
        let conn = TcpStream::connect("127.0.0.1:8080").await.unwrap();

        let conn = tokio_smux::Mux::new(conn, tokio_smux::Config::new(1, 1, 16192 as usize));
        let mut stream = conn.connect().await.unwrap();
        let mut buf = [0u8; 8092];
        let mut total = 0;
        while total < 1024 * 1024 * 256 {
            let n = stream.write(&mut buf).await.unwrap();
            total += n;
        }
        
        _ = read_handle.await;
    });

    _ = write_handle.await;
}

async fn transfer_raw() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();

    let read_handle = tokio::spawn(async move {
        let (mut conn, _) = listener.accept().await.unwrap();
        let mut buf = [0u8; 8092];
        let mut total = 0;
        while total < 1024 * 1024 * 256 {
        let n = conn.read(&mut buf).await.unwrap();
            total += n;
        }
        drop(listener);
    });

    let write_handle = tokio::spawn(async move {
        let mut conn = TcpStream::connect("127.0.0.1:8080").await.unwrap();

        let mut buf = [0u8; 8092];
        let mut total = 0;
        while total < 1024 * 1024 * 256 {
            let n = conn.write(&mut buf).await.unwrap();
            total += n;
        }
        
        _ = read_handle.await;
    });

    _ = write_handle.await;
}

#[tokio::main]
async fn bench_roundtrip_raw(b: &mut Bencher){
    b.iter(|| {
        futures::executor::block_on(roundtrip_raw());
    })
}

#[tokio::main]
async fn bench_roundtrip_mux(b: &mut Bencher){
    b.iter(|| {
        futures::executor::block_on(roundtrip_mux());
    })
}

#[tokio::main]
async fn bench_transfer_raw(b: &mut Bencher){
    b.iter(|| {
        futures::executor::block_on(transfer_raw());
    })
}

#[tokio::main]
async fn bench_transfer_mux(b: &mut Bencher){
    b.iter(|| {
        futures::executor::block_on(transfer_mux());
    })
}


benchmark_group!(benches, bench_roundtrip_raw, bench_roundtrip_mux, bench_transfer_mux, bench_transfer_raw);
benchmark_main!(benches);