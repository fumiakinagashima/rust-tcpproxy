use rand::RngExt;
use std::time::Duration;

pub struct FaultInjector {
    delay_probability: f64,
    delay_duration: Duration,
    disconnect_probability: f64,
}

impl FaultInjector {
    pub fn new(
        delay_probability: f64,
        delay_duration: Duration,
        disconnect_probability: f64
    ) -> Self {
        Self {
            delay_probability,
            delay_duration,
            disconnect_probability,
        }
    }

    pub async fn maybe_delay(&self) {
        if rand::rng().random::<f64>() < self.delay_probability {
            tokio::time::sleep(self.delay_duration).await;
        }
    }

    pub fn should_disconnect(&self) -> bool {
        rand::rng().random::<f64>() < self.disconnect_probability
    }
}