use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpStream;

pub struct Pool {
    addr: SocketAddr,
    idle: Mutex<VecDeque<TcpStream>>,
}

impl Pool {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            idle: Mutex::new(VecDeque::new()),
        }
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn try_get(&self) -> Option<TcpStream> {
        self.idle.lock().unwrap().pop_front()
    }

    pub fn put(&self, stream: TcpStream) {
        self.idle.lock().unwrap().push_back(stream);
    }

    pub fn len(&self) -> usize {
        self.idle.lock().unwrap().len()
    }
}

pub fn new_pools(addrs: &[SocketAddr]) -> Vec<Arc<Pool>> {
    addrs.iter().map(|&addr| Arc::new(Pool::new(addr))).collect()
}

pub async fn run_pool_filler(pool: Arc<Pool>, target_size: usize, refill_interval: Duration) {
    let mut ticker = tokio::time::interval(refill_interval);
    loop {
        ticker.tick().await;
        let deficit = target_size.saturating_sub(pool.len());
        for _ in 0..deficit {
            if let Ok(stream) = TcpStream::connect(pool.addr()).await {
                pool.put(stream);
            }
        }
    }
}