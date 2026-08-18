use crate::health::Backends;
use crate::pool::Pool;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub enum RejectReason {
    IpRateLimit,
    ConnectionLimit,
}

pub enum Direction {
    ClientToBackend,
    BackendToClient,
}

pub struct Metrics {
    connections_total: AtomicU64,
    connections_active: AtomicU64,
    rejected_ip_rate_limit_total: AtomicU64,
    rejected_connection_limit_total: AtomicU64,
    bytes_client_to_backend_total: AtomicU64,
    bytes_backend_to_client_total: AtomicU64,
    backend_addrs: Vec<SocketAddr>,
    backend_failures_total: Vec<AtomicU64>,
}

impl Metrics {
    pub fn new(backend_addrs: Vec<SocketAddr>) -> Self {
        let backend_failures_total = backend_addrs.iter().map(|_| AtomicU64::new(0)).collect();
        Self {
            connections_total: AtomicU64::new(0),
            connections_active: AtomicU64::new(0),
            rejected_ip_rate_limit_total: AtomicU64::new(0),
            rejected_connection_limit_total: AtomicU64::new(0),
            bytes_client_to_backend_total: AtomicU64::new(0),
            bytes_backend_to_client_total: AtomicU64::new(0),
            backend_addrs,
            backend_failures_total,
        }
    }

    pub fn track_connection(&self) -> ActiveGuard<'_> {
        self.connections_total.fetch_add(1, Ordering::Relaxed);
        self.connections_active.fetch_add(1, Ordering::Relaxed);
        ActiveGuard { metrics: self }
    }

    pub fn inc_rejected(&self, reason: RejectReason) {
        let counter = match reason {
            RejectReason::IpRateLimit => &self.rejected_ip_rate_limit_total,
            RejectReason::ConnectionLimit => &self.rejected_connection_limit_total,
        };
        counter.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_bytes(&self, direction: Direction, n: u64) {
        let counter = match direction {
            Direction::ClientToBackend => &self.bytes_client_to_backend_total,
            Direction::BackendToClient => &self.bytes_backend_to_client_total,
        };
        counter.fetch_add(n, Ordering::Relaxed);
    }

    pub fn inc_backend_failure(&self, idx: usize) {
        self.backend_failures_total[idx].fetch_add(1, Ordering::Relaxed);
    }

    pub fn render(&self, backends: &Backends, pools: &[Arc<Pool>]) -> String {
        let mut out = String::new();

        out.push_str("# HELP tcpproxy_connections_total Total number of accepted connections. \n");
        out.push_str("# TYPE tcpproxy_connections_total counter\n");
        out.push_str(&format!(
            "tcpproxy_connections_total {}\n\n",
            self.connections_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP tcpproxy_connections_active Connections currently being proxied. \n");
        out.push_str("# TYPE tcpproxy_connections_active gauge\n");
        out.push_str(&format!(
            "tcpproxy_connections_active {}\n\n",
            self.connections_active.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP tcpproxy_rejected_total Connections rejected before proxyin, by reason.\n");
        out.push_str("# TYPE tcpproxy_rejected_total counter\n");
        out.push_str(&format!(
            "tcpproxy_rejected_total{{reason=\"ip_rate_limit\"}} {}\n",
            self.rejected_ip_rate_limit_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "tcpproxy_rejected_total{{reason=\"connection_limit\"}} {}\n\n",
            self.rejected_connection_limit_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP tcpproxy_bytes_total Bytes transferred, by direction.\n");
        out.push_str("# TYPE tcpproxy_bytes_total counter\n");
        out.push_str(&format!(
            "tcpproxy_bytes_total{{direction=\"client_to_backend\"}} {}\n",
            self.bytes_client_to_backend_total.load(Ordering::Relaxed)
        ));
        out.push_str(&format!(
            "tcpproxy_bytes_total{{direction=\"backend_to_client\"}} {}\n\n",
            self.bytes_backend_to_client_total.load(Ordering::Relaxed)
        ));

        out.push_str("# HELP tcpproxy_backend_up Whether the backend is currently considered healthy.\n");
        out.push_str("# TYPE tcpproxy_backend_up gauge\n");
        for (idx, addr) in self.backend_addrs.iter().enumerate() {
            out.push_str(&format!(
                "tcpproxy_backend_up{{backend=\"{addr}\"}} {}\n",
                backends.is_healthy(idx) as u8
            ));
        }
        out.push('\n');

        out.push_str("# HELP tcpproxy_backend_failures_total Backend connect failures recorded, by backend.\n");
        out.push_str("# TYPE tcpproxy_backend_failures_total counter\n");
        for (idx, addr) in self.backend_addrs.iter().enumerate() {
            out.push_str(&format!(
                "tcpproxy_backend_failures_total{{backend=\"{addr}\"}} {}\n",
                self.backend_failures_total[idx].load(Ordering::Relaxed)
            ));
        }
        out.push('\n');

        out.push_str("# HELP tcpproxy_pool_idle_connections Idle pooled connections currently held, by backend.\n");
        out.push_str("# TYPE tcpproxy_pool_idle_connections gauge\n");
        for pool in pools {
            out.push_str(&format!(
                "tcpproxy_pool_idle_connections{{backend=\"{}\"}} {}\n",
                pool.addr(),
                pool.len()
            ));
        }

        out
    }
}

pub struct ActiveGuard<'a> {
    metrics: &'a Metrics,
}

impl Drop for ActiveGuard<'_> {
    fn drop(&mut self) {
        self.metrics.connections_active.fetch_sub(1, Ordering::Relaxed);
    }
}