use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

use crate::health::Backends;
use crate::load_balancer::LoadBalancer;
use crate::pool::Pool;

pub async fn handle_connection(
    mut inbound: TcpStream,
    peer_addr: SocketAddr,
    lb: Arc<dyn LoadBalancer>,
    backends: Arc<Backends>,
    pools: Arc<Vec<Arc<Pool>>>,
    failure_threshold: usize,
) -> std::io::Result<()> {
    let Some(backend_addr) = lb.next_backend() else {
        return Err(std::io::Error::other("no health backend available"));
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
                }
                lb.release(backend_addr);
                return Err(e);
            }
        }
    };

    let result = copy_bidirectional(&mut inbound, &mut outbound).await;
    lb.release(backend_addr);
    
    let (from_client, from_backend) = result?;
    println!(
        "{peer_addr} -> {backend_addr}: {from_client} bytes client->backend, {from_backend} bytes backend->client"
    );
    
    Ok({})
}