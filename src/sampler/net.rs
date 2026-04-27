use std::time::Instant;

use sysinfo::Networks;

#[derive(Debug, Clone, Default)]
pub struct NetSample {
    pub rx_bps: u64,
    pub tx_bps: u64,
}

pub struct NetSampler {
    networks: Networks,
    last: Option<(Instant, u64, u64)>,
}

impl NetSampler {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            last: None,
        }
    }

    pub fn tick(&mut self) -> NetSample {
        self.networks.refresh();
        let now = Instant::now();
        let (mut rx, mut tx) = (0u64, 0u64);
        for (name, n) in self.networks.iter() {
            if name == "lo" || name.starts_with("docker") || name.starts_with("br-") {
                continue;
            }
            rx = rx.saturating_add(n.total_received());
            tx = tx.saturating_add(n.total_transmitted());
        }
        match self.last.replace((now, rx, tx)) {
            Some((prev_t, prev_rx, prev_tx)) => {
                let dt = now.saturating_duration_since(prev_t).as_secs_f64().max(0.001);
                NetSample {
                    rx_bps: ((rx.saturating_sub(prev_rx) as f64) / dt) as u64,
                    tx_bps: ((tx.saturating_sub(prev_tx) as f64) / dt) as u64,
                }
            }
            None => NetSample::default(),
        }
    }
}

impl Default for NetSampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tick_is_zero() {
        let mut s = NetSampler::new();

        let first = s.tick();

        assert_eq!(first.rx_bps, 0);
        assert_eq!(first.tx_bps, 0);
    }
}
