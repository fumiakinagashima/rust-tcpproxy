mod least_connections;
mod random;
mod round_robin;
mod weighted;

pub use least_connections::LeastConnections;
pub use random::Random;
pub use round_robin::RoundRobin;
pub use weighted::Weighted;

use std::net::SocketAddr;

pub trait LoadBalancer: Send + Sync {
    fn next_backend(&self) -> SocketAddr;
    fn release(&self, _backend_addr: SocketAddr) {}
}
