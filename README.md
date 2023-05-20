# tokio-tcpmux

A Tokio-compatible TCP multiplexing library with a focus on speed and reliability.

![img](https://raw.githubusercontent.com/rob-maron/tokio-tcpmux/master/mux.png)

## Benchmarks
|       Implementation     |  Throughput (TCP)  |
| ------------------------ | ------------------ |
| **tokio-tcpmux**         | **3.6352 GiB/s**   |

## Example
```rust
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tcpmux::{Config, Mux};

async fn echo_server() {
    // listen for new connection
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let (conn, _) = listener.accept().await.unwrap();

    // wrap connection in the multiplexer
    let conn = Mux::new(conn, Config::default());

    loop {
        // virtual accept
        let mut stream = conn.accept().await.unwrap();

        tokio::spawn(async move {
            // read string
            let len = stream.read_u64().await.unwrap() as usize;
            let mut buf = vec![0; len];
            stream.read_exact(&mut buf).await.unwrap();

            // send string back
            stream.write_u64(buf.len() as u64).await.unwrap();
            stream.write_all(&buf).await.unwrap();

            println!("Echo server received: {}", String::from_utf8(buf).unwrap());
        });
    }
}

#[tokio::main]
async fn main() {
    // spawn server
    _ = tokio::spawn(echo_server());

    // create and wrap client
    let conn = TcpStream::connect("127.0.0.1:8080").await.unwrap();
    let conn = Mux::new(conn, Config::default());

    for i in 0..100 {
        // virtual connect
        let mut conn = conn.connect().await.unwrap();

        // send string OTW
        let string_to_send = format!("Hello from {}", i);
        conn.write_u64(string_to_send.len() as u64).await.unwrap();
        conn.write_all(string_to_send.as_bytes()).await.unwrap();

        // read string
        let len = conn.read_u64().await.unwrap() as usize;
        let mut buf = vec![0; len];
        conn.read_exact(&mut buf).await.unwrap();


        println!("Echo client received: {}", String::from_utf8(buf).unwrap());
    }
}

```
