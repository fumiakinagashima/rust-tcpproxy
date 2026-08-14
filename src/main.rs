mod health;
mod load_balancer;
mod proxy;

use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

use health::{run_health_checks, Backends};
use load_balancer::{LoadBalancer, RoundRobin};
use proxy::handle_connection;

const LISTEN_ADDR: &str = "127.0.0.1:8000";
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(3);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let addrs = vec![
        "127.0.0.1:9001".parse().unwrap(),
        "127.0.0.1:9002".parse().unwrap(),
        "127.0.0.1:9003".parse().unwrap(),
    ];
    let backends = Arc::new(Backends::new(addrs));

    tokio::spawn(run_health_checks(
        Arc::clone(&backends),
        HEALTH_CHECK_INTERVAL,
        HEALTH_CHECK_TIMEOUT,
    ));

    let lb: Arc<dyn LoadBalancer> = Arc::new(RoundRobin::new(Arc::clone(&backends)));

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
