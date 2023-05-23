# tokio-tcpmux

A Tokio-compatible TCP multiplexing library with a focus on speed and reliability

![img](https://raw.githubusercontent.com/rob-maron/tokio-tcpmux/master/mux.png)

## Benchmarks
|       Implementation     |  Throughput (TCP)            |   Connect Roundtrip Time          |
| ------------------------ | ---------------------------- | --------------------------------- |
|     **tokio-tcpmux**     |  **3.6352 GiB/s** (-2.86x)   |         **992,938ns** (+5.61%)    |
|      vanilla stream      |   10.4098 GiB/s              |         938,750ns                 |

These benchmarks were run on an Intel Core i7-12700KF. They are available for use by anyone under the `benches/` directory.


## Example
```rust
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};
use tokio_tcpmux::{Config, Mux};

async fn echo_server() {
    // listen for new connection
    let listener = TcpListener::bind("127.0.0.1:8081").await.unwrap();
    let (conn, _) = listener.accept().await.unwrap();

    // wrap connection in the multiplexer
    let conn = Mux::new(conn, Config::default());

    loop {
        // virtual accept
        let mut stream = conn.accept().await.unwrap();

        tokio::spawn(async move {
            // read string
            let mut buf = vec![0; 100];
            stream.read(&mut buf).await.unwrap();

            // send string back
            stream.write_all(&buf).await.unwrap();

            println!("Echo server received: {}", String::from_utf8(buf).unwrap());
        });
    }
}

#[tokio::main]
async fn main() {
    // spawn server
    _ = tokio::spawn(echo_server());
    
    // wait a second for server to start (not needed in real code)
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // create and wrap client
    let conn = TcpStream::connect("127.0.0.1:8081").await.unwrap();
    let conn = Mux::new(conn, Config::default());

    for i in 0..100 {
        // virtual connect
        let mut conn = conn.connect().await.unwrap();

        // send string OTW
        let string_to_send = format!("Hello from {}", i);
        conn.write_all(string_to_send.as_bytes()).await.unwrap();

        // read string
        let mut buf = vec![0; 100];
        conn.read(&mut buf).await.unwrap();

        println!("Echo client received: {}", String::from_utf8(buf).unwrap());
    }
}
```
