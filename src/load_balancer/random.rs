use std::net::SocketAddr;
use std::sync::Arc;
use rand::RngExt;

use crate::health::Backends;

use super::LoadBalancer;

pub struct Random {
    backends: Arc<Backends>,
}

impl Random {
    pub fn new(backends: Arc<Backends>) -> Self {
        Self { backends }
    }
}

impl LoadBalancer for Random {
    fn next_backend(&self) -> Option<SocketAddr> {
        let healthy_indices: Vec<usize> = (0..self.backends.addrs().len())
            .filter(|&idx| self.backends.is_healthy(idx))
            .collect();
        if healthy_indices.is_empty() {
            return None;
        }
        let pick = rand::rng().random_range(0..healthy_indices.len());
        Some(self.backends.addrs()[healthy_indices[pick]]) 
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_picks_one_of_the_configured_backends() {
        let backends: Vec<SocketAddr> = vec![
            "127.0.0.1:9001".parse().unwrap(),
            "127.0.0.1:9002".parse().unwrap(),
        ];
        let lb = Random::new(backends.clone());

        for _ in 0..50 {
            assert!(backends.contains(&lb.next_backend()));
        }
    }
}