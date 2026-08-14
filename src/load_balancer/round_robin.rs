use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::LoadBalancer;

pub struct RoundRobin {
    backends: Vec<SocketAddr>,
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new(backends: Vec<SocketAddr>) -> Self {
        Self {
            backends,
            counter: AtomicUsize::new(0),
        }
    }
}

impl LoadBalancer for RoundRobin {
    fn next_backend(&self) -> SocketAddr {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.backends.len();
        self.backends[idx]
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