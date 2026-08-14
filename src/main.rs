use std::net::SocketAddr;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};

const LISTEN_ADDR: &str = "127.0.0.1:8000";
const BACKEND_ADDR: &str = "127.0.0.1:9000";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    println!("listening on {LISTEN_ADDR}, forwarding to {BACKEND_ADDR}");
    loop {
        let (inbound, peer_addr) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound, peer_addr).await {
                eprintln!("connection error ({peer_addr}): {e}");
            }
        });
    }
}

async fn handle_connection(mut inbound: TcpStream, peer_addr: SocketAddr) -> std::io::Result<()> {
    let mut outbound = TcpStream::connect(BACKEND_ADDR).await?;
    let (from_client, from_backend) = copy_bidirectional(&mut inbound, &mut outbound).await?;
    println!("{peer_addr}: {from_backend} bytes client->backend, {from_backend} bytes backend->client");
    
    Ok({})
}