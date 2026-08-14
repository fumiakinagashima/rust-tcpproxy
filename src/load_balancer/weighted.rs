use std::net::SocketAddr;
use std::sync::Mutex;

use super::LoadBalancer;

struct WeightedBackend {
    addr: SocketAddr,
    weight: i32,
    current_weight: i32,
}

pub struct Weighted {
    backends: Mutex<Vec<WeightedBackend>>,
    total_weight: i32,
}

impl Weighted {
    pub fn new(backends: Vec<(SocketAddr, i32)>) -> Self {
        let total_weight = backends.iter().map(|(_, weight)| weight).sum();
        let backends = backends
            .into_iter()
            .map(|(addr, weight)| WeightedBackend {
                addr,
                weight,
                current_weight: 0,
            })
            .collect();
        Self {
            backends: Mutex::new(backends),
            total_weight,
        }
    }
}

impl LoadBalancer for Weighted {
    fn next_backend(&self) -> SocketAddr {
        let mut backends = self.backends.lock().unwrap();
        for b in backends.iter_mut() {
            b.current_weight += b.weight;
        }
        let selected = backends
            .iter_mut()
            .max_by_key(|b| b.current_weight)
            .unwrap();
        selected.current_weight -= self.total_weight;
        selected.addr
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distributes_backends_proportionally_to_weight() {
        let a: SocketAddr = "127.0.0.1:9001".parse().unwrap();
        let b: SocketAddr = "127.0.0.1:9002".parse().unwrap();
        let c: SocketAddr = "127.0.0.1:9003".parse().unwrap();
        let lb = Weighted::new(vec![(a, 5), (b, 1), (c, 1)]);

        let picked: Vec<SocketAddr> = (0..7).map(|_| lb.next_backend()).collect();

        assert_eq!(picked.iter().filter(|&&addr| addr == a).count(), 5);
        assert_eq!(picked.iter().filter(|&&addr| addr == b).count(), 1);
        assert_eq!(picked.iter().filter(|&&addr| addr == c).count(), 1);
    }
}