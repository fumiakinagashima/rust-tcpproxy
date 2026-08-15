use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;

pub struct Backends {
    addrs: Vec<SocketAddr>,
    healthy: Vec<AtomicBool>,
    consecutive_failures: Vec<AtomicUsize>,
}

impl Backends {
    pub fn new(addrs: Vec<SocketAddr>) -> Self {
        let healthy = addrs.iter().map(|_| AtomicBool::new(true)).collect();
        let consecutive_failures = addrs.iter().map(|_| AtomicUsize::new(0)).collect();
        Self { addrs, healthy, consecutive_failures }
    }

    pub fn addrs(&self) -> &[SocketAddr] {
        &self.addrs
    }

    pub fn is_healthy(&self, idx: usize) -> bool {
        self.healthy[idx].load(Ordering::Relaxed)
    }

    pub fn record_success(&self, idx: usize) {
        self.consecutive_failures[idx].store(0, Ordering::Relaxed);
    }

    pub fn record_failure(&self, idx: usize, threshold: usize) {
        let failures = self.consecutive_failures[idx].fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= threshold {
            self.set_healthy(idx, false);
        }
    }

    pub fn index_of(&self, addr: SocketAddr) -> Option<usize> {
        self.addrs.iter().position(|&a| a == addr)
    }

    fn set_healthy(&self, idx: usize, healthy: bool) {
        self.healthy[idx].store(healthy, Ordering::Relaxed);
    }
}

pub async fn run_health_checks(
    backends: Arc<Backends>,
    interval: Duration,
    timeout: Duration
) {
    let mut ticket =tokio::time::interval(interval);
    loop {
        ticket.tick().await;
        for idx in 0..backends.addrs().len() {
            let backends = Arc::clone(&backends);
            tokio::spawn(async move {
                let addr = backends.addrs()[idx];
                let healthy = tokio::time::timeout(
                    timeout,
                    TcpStream::connect(addr)
                )
                    .await
                    .is_ok_and(|connect_result| connect_result.is_ok());
                backends.set_healthy(idx, healthy);
            });
        }
    }
}