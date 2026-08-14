mod load_balancer;
mod proxy;

use std::sync::Arc;
use tokio::net::TcpListener;
use load_balancer::{LoadBalancer, Weighted};
use proxy::handle_connection;

const LISTEN_ADDR: &str = "127.0.0.1:8000";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let backends = vec![
        ("127.0.0.1:9001".parse().unwrap(), 5),
        ("127.0.0.1:9002".parse().unwrap(), 1),
        ("127.0.0.1:9003".parse().unwrap(), 1),
    ];
    let lb: Arc<dyn LoadBalancer> = Arc::new(Weighted::new(backends));

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    println!("listening on {LISTEN_ADDR}");

    loop {
        let (inbound, peer_addr) = listener.accept().await?;
        let lb = Arc::clone(&lb);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound, peer_addr, lb).await {
                eprintln!("connection error ({peer_addr}): {e}");
            }
        });
    }
}
