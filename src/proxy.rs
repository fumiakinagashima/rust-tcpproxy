use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::TcpStream;

use crate::load_balancer::LoadBalancer;

async fn forward(
    inbound: &mut TcpStream,
    backend_addr: SocketAddr
) -> std::io::Result<(u64, u64)> {
    let mut outbound = TcpStream::connect(backend_addr).await?;
    copy_bidirectional(inbound, &mut outbound).await
}

pub async fn handle_connection(
    mut inbound: TcpStream,
    peer_addr: SocketAddr,
    lb: Arc<dyn LoadBalancer>,
) -> std::io::Result<()> {
    let Some(backend_addr) = lb.next_backend() else {
        return Err(std::io::Error::other("no health backend available"));
    };

    let result = forward(&mut inbound, backend_addr).await;
    lb.release(backend_addr);

    let (from_client, from_backend) = result?;
    println!(
        "{peer_addr} -> {backend_addr}: {from_client} bytes client->backend, {from_backend} bytes backend->client"
    );
    
    Ok({})
}