use std::net::SocketAddr;
use rand::RngExt;

use super::LoadBalancer;

pub struct Random {
    backends: Vec<SocketAddr>,
}

impl Random {
    pub fn new(backends: Vec<SocketAddr>) -> Self {
        Self { backends }
    }
}

impl LoadBalancer for Random {
    fn next_backend(&self) -> SocketAddr {
        let idx = rand::rng().random_range(0..self.backends.len());
        self.backends[idx]
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