use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::health::Backends;

use super::LoadBalancer;

pub struct LeastConnections {
    backends: Arc<Backends>,
    active: Vec<AtomicUsize>,
}

impl LeastConnections {
    pub fn new(backends: Arc<Backends>) -> Self {
        let active = backends.addrs().iter().map(|_| AtomicUsize::new(0)).collect();
        Self { backends, active }
    }
}

impl LoadBalancer for LeastConnections {
    fn next_backend(&self) -> Option<SocketAddr> {
        let idx = self
            .active
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.backends.is_healthy(*idx))
            .min_by_key(|(_, count)| count.load(Ordering::Relaxed))
            .map(|(idx, _)| idx)?;
        self.active[idx].fetch_add(1, Ordering::Relaxed);
        Some(self.backends.addrs()[idx])
    }

    fn release(&self, backend_addr: SocketAddr) {
        if let Some(idx) = self.backends.addrs().iter().position(|&b| b == backend_addr) {
            self.active[idx].fetch_sub(1, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avoids_a_backend_that_is_still_active() {
        let backends: Vec<SocketAddr> = vec![
            "127.0.0.1:9001".parse().unwrap(),
            "127.0.0.1:9002".parse().unwrap(),
        ];
        let lb = LeastConnections::new(backends.clone());

        let busy = lb.next_backend();
        let picked = lb.next_backend();

        assert_ne!(busy, picked);
    }

    #[test]
    fn released_backend_becomes_eligible_again() {
        let backends: Vec<SocketAddr> = vec![
            "127.0.0.1:9001".parse().unwrap(),
            "127.0.0.1:9002".parse().unwrap(),
        ];
        let lb = LeastConnections::new(backends.clone());

        let first = lb.next_backend();
        lb.release(first);
        let second = lb.next_backend();

        assert_eq!(first, second);
    }
}