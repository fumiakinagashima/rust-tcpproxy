use crate::health::Backends;
use crate::metrics::Metrics;
use crate::pool::Pool;

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

pub async fn run_metrics_server(
    addr: &str,
    metrics: Arc<Metrics>,
    backends: Arc<Backends>,
    pools: Arc<Vec<Arc<Pool>>>,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr).await?;
    println!("metrics listening on {addr}");

    loop {
        let (stream, _) = listener.accept().await?;
        let metrics = Arc::clone(&metrics);
        let backends = Arc::clone(&backends);
        let pools = Arc::clone(&pools);
        tokio::spawn(async move {
            if let Err(e) = serve(stream, &metrics, &backends, &pools).await {
                eprintln!("metrics request error: {e}");
            }
        });
    }   
}

async fn serve(
    stream: TcpStream,
    metrics: &Metrics,
    backends: &Backends,
    pools: &[Arc<Pool>],
) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);

    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 || line == "\r\n" {
            break;
        }
    }

    let body = metrics.render(backends, pools);
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: clone\r\n\r\n{}",
        body.len(),
        body
    );

    let mut stream = reader.into_inner();
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}