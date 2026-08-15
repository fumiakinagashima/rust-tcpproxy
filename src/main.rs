mod health;
mod load_balancer;
mod pool;
mod proxy;

use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;

use health::{run_health_checks, Backends};
use load_balancer::{LoadBalancer, RoundRobin};
use pool::{new_pools, run_pool_filler};
use proxy::handle_connection;

const LISTEN_ADDR: &str = "127.0.0.1:8000";
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(60);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(1);
const FAILURE_THRESHOLD: usize = 3;
const POOL_TARGET_SIZE: usize = 4;
const POOL_REFILL_INTERVAL: Duration = Duration::from_millis(200);

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

    let pools = Arc::new(new_pools(backends.addrs()));
    for pool in pools.iter() {
        tokio::spawn(run_pool_filler(
            Arc::clone(pool),
            POOL_TARGET_SIZE,
            POOL_REFILL_INTERVAL,
        ));
    }

    let lb: Arc<dyn LoadBalancer> = Arc::new(RoundRobin::new(Arc::clone(&backends)));

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    println!("listening on {LISTEN_ADDR}");

    loop {
        let (inbound, peer_addr) = listener.accept().await?;
        let lb = Arc::clone(&lb);
        let backends = Arc::clone(&backends);
        let pools = Arc::clone(&pools);
        tokio::spawn(async move {
            if let Err(e) = handle_connection(
                inbound,
                peer_addr,
                lb,
                backends,
                pools,
                FAILURE_THRESHOLD
            ).await {
                eprintln!("connection error ({peer_addr}): {e}");
            }
        });
    }
}
