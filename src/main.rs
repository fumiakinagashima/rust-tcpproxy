mod health;
mod load_balancer;
mod pool;
mod proxy;

use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::JoinSet;

use health::{run_health_checks, Backends};
use load_balancer::{LoadBalancer, RoundRobin};
use pool::{new_pools, run_pool_filler};
use proxy::handle_connection;

const LISTEN_ADDR: &str = "127.0.0.1:8000";
const HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(3);
const HEALTH_CHECK_TIMEOUT: Duration = Duration::from_secs(1);
const FAILURE_THRESHOLD: usize = 3;
const POOL_TARGET_SIZE: usize = 4;
const POOL_REFILL_INTERVAL: Duration = Duration::from_millis(200);
const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_CONNECTION_ATTEMPTS: usize = 3;

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

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut tasks = JoinSet::new();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (inbound, peer_addr) = accepted?;
                let lb = Arc::clone(&lb);
                let backends = Arc::clone(&backends);
                let pools = Arc::clone(&pools);
                tasks.spawn(async move {
                    if let Err(e) = handle_connection(
                        inbound,
                        peer_addr,
                        lb,
                        backends,
                        pools,
                        FAILURE_THRESHOLD,
                        MAX_CONNECTION_ATTEMPTS
                    ).await {
                        eprintln!("connection error ({peer_addr}): {e}");
                    }
                });
            }
            _ = sigterm.recv() => {
                println!("received SIGTERM, no longer accepting new connections");
                break;
            }
            _ = sigint.recv() => {
                println!("received Ctrl+C, no longer accepting new connection");
                break;
            }
        }
    }

    println!("draining {} in-flight connection(s)", tasks.len());
    let drain = async {
        while tasks.join_next().await.is_some() {}
    };
    tokio::select! {
        _ = drain => {
            println!("all connections drained, shutdown complete");
        }
        _ = tokio::time::sleep(SHUTDOWN_DRAIN_TIMEOUT) => {
            eprintln!(
                "drain timed out after {SHUTDOWN_DRAIN_TIMEOUT:?}, exiting with {} connection(s) still in flight",
                tasks.len()
            );
        }
        _ = sigterm.recv() => {
            eprintln!("received a second SIGTERM during drain, forcing immediate shutdown");
        }
        _ = sigint.recv() => {
            eprintln!("received a second Ctrl+C during drain, forcing immediate shutdown");
        }
    }

    Ok({})
}
