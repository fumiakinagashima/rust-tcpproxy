use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::health::Backends;

use super::LoadBalancer;

struct WeightedState {
    weight: i32,
    current_weight: i32,
}

pub struct Weighted {
    backends: Arc<Backends>,
    state: Mutex<Vec<WeightedState>>,
}

impl Weighted {
    pub fn new(backends: Arc<Backends>, weights: Vec<i32>) -> Self {
        assert_eq!(backends.addrs().len(), weights.len());
        let state = weights
            .into_iter()
            .map(|weight| WeightedState {
                weight,
                current_weight: 0,
            })
            .collect();
        Self {
            backends,
            state: Mutex::new(state),
        }
    }
}

impl LoadBalancer for Weighted {
    fn next_backend(&self) -> Option<SocketAddr> {
        let mut state = self.state.lock().unwrap();
        let total_weight: i32 = state
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.backends.is_healthy(*idx))
            .map(|(_, s)| s.weight)
            .sum();
        if total_weight == 0 {
            return None;
        }

        for (idx, s) in state.iter_mut().enumerate() {
            if self.backends.is_healthy(idx) {
                s.current_weight += s.weight;
            }
        }

        let (idx, selected) = state
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| self.backends.is_healthy(*idx))
            .max_by_key(|(_, s)| s.current_weight)?;
        selected.current_weight -= total_weight;
        Some(self.backends.addrs()[idx])        
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