use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::health::Backends;

use super::LoadBalancer;

pub struct RoundRobin {
    backends: Arc<Backends>,
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new(backends: Arc<Backends>) -> Self {
        Self {
            backends,
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn next_backend(&self) -> Option<SocketAddr> {
        let healthy_indices: Vec<usize> = (0..self.backends.addrs().len())
            .filter(|&idx| self.backends.is_healthy(idx))
            .collect();
        if healthy_indices.is_empty() {
            return None;
        }
        let pick = self.counter.fetch_add(1, Ordering::Relaxed) % healthy_indices.len();
        Some(self.backends.addrs()[healthy_indices[pick]]) 
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycles_through_all_backends_in_order() {
        let backends: Vec<SocketAddr> = vec![
            "127.0.0.1:9001".parse().unwrap(),
            "127.0.0.1:9002".parse().unwrap(),
            "127.0.0.1:9003".parse().unwrap(),
        ];
        let lb = RoundRobin::new(backends.clone());

        let picked: Vec<SocketAddr> = (0..6).map(|_| lb.next_backend()).collect();

        assert_eq!(
            picked,
            vec![
                backends[0], backends[1], backends[2],
                backends[0], backends[1], backends[2],
            ]
        );
    }
}