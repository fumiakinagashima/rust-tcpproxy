use crate::health::Backends;
use crate::load_balancer::LoadBalancer;
use crate::pool::Pool;
use crate::proxy_protocol::v2_header;
use crate::tls_sni::peek_sni;
use crate::metrics::{Direction, Metrics};

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;
use tokio::io::AsyncWriteExt;
use std::collections::HashMap;

pub async fn handle_connection(
    mut inbound: TcpStream,
    peer_addr: SocketAddr,
    lb: Arc<dyn LoadBalancer>,
    backends: Arc<Backends>,
    pools: Arc<Vec<Arc<Pool>>>,
    sni_routes: Arc<HashMap<String, SocketAddr>>,
    sni_peek_timeout: Duration,
    failure_threshold: usize,
    max_connection_attempts: usize,
    metrics: Arc<Metrics>,
) -> std::io::Result<()> {
    let _active = metrics.track_connection();
    let sni_backend = peek_sni(&inbound, sni_peek_timeout)
        .await
        .and_then(|name| sni_routes.get(&name).copied());
    let mut last_err = std::io::Error::other("no healthy backend available");

    for _ in 0..max_connection_attempts {
        let backend_addr = match sni_backend {
            Some(addr) => addr,
            None => match lb.next_backend() {
                Some(addr) => addr,
                None => return Err(last_err),
            },
        };
        
        let idx = backends.index_of(backend_addr);

        let mut outbound = if let Some(stream) = idx.and_then(|idx| pools[idx].try_get()) {
            stream
        } else {
            match TcpStream::connect(backend_addr).await {
                Ok(stream) => {
                    if let Some(idx) = idx {
                        backends.record_success(idx);
                    }
                    stream
                }
                Err(e) => {
                    if let Some(idx) = idx {
                        backends.record_failure(idx, failure_threshold);
                        metrics.inc_backend_failure(idx);
                    }
                    if sni_backend.is_none() {
                        lb.release(backend_addr);
                    }
                    last_err = e;
                    continue;
                }
            }
        };

        let local_addr = outbound.local_addr()?;
        let header = v2_header(peer_addr, local_addr);
        outbound.write_all(&header).await?;

        let result = copy_bidirectional(&mut inbound, &mut outbound).await;
        if sni_backend.is_none() {
            lb.release(backend_addr);
        }
        
        let (from_client, from_backend) = result?;
        metrics.add_bytes(Direction::ClientToBackend, from_client);
        metrics.add_bytes(Direction::BackendToClient, from_backend);
        println!(
            "{peer_addr} -> {backend_addr}: {from_client} bytes client->backend, {from_backend} bytes backend->client"
        );
        return Ok(());
    }

    Err(last_err)
}
