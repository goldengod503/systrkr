use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{GpuBackend, GpuSample};

pub struct IntelSysfs {
    card_path: PathBuf,
    name: String,
    pdev: String,
    last: Option<(Instant, u64)>,
}

impl IntelSysfs {
    pub fn probe_in(drm_root: &Path) -> Option<Self> {
        let entries = fs::read_dir(drm_root).ok()?;
        for entry in entries.flatten() {
            let card = entry.path();
            if !card
                .file_name()
                .and_then(|n| n.to_str())
                .map(super::is_card_dir)
                .unwrap_or(false)
            {
                continue;
            }
            if let Some(backend) = Self::probe_specific(&card) {
                return Some(backend);
            }
        }
        None
    }

    pub(crate) fn probe_specific(card: &Path) -> Option<Self> {
        let device = card.join("device");
        if !is_intel(&device) {
            return None;
        }
        if find_render_engine(card).is_none() {
            return None;
        }
        let name = read_intel_name(&device);
        let pdev = fs::read_link(&device)
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
            .unwrap_or_default();
        Some(Self {
            card_path: card.to_path_buf(),
            name,
            pdev,
            last: None,
        })
    }

    pub fn probe_pdev(pdev: &str) -> Option<Self> {
        super::resolve_card_by_pdev(pdev).and_then(|card| Self::probe_specific(&card))
    }
}

impl GpuBackend for IntelSysfs {
    fn name(&self) -> &str {
        &self.name
    }

    fn pdev(&self) -> &str {
        &self.pdev
    }

    fn is_nvidia(&self) -> bool {
        false
    }

    fn sample(&mut self) -> GpuSample {
        let utilization_pct = self.read_utilization();
        GpuSample {
            utilization_pct,
            memory_used_bytes: None,
            memory_total_bytes: None,
            temperature_c: None,
        }
    }
}

impl IntelSysfs {
    fn read_utilization(&mut self) -> Option<f32> {
        let engine = find_render_engine(&self.card_path)?;
        let raw = fs::read_to_string(engine.join("busy")).ok()?;
        let busy_us: u64 = raw.trim().parse().ok()?;
        let now = Instant::now();
        let prev = self.last.replace((now, busy_us));
        let (prev_t, prev_busy) = prev?;
        let dt_us = u64::try_from(now.saturating_duration_since(prev_t).as_micros())
            .unwrap_or(u64::MAX);
        if dt_us == 0 {
            return None;
        }
        let dbusy = busy_us.saturating_sub(prev_busy);
        let pct = (dbusy as f64 / dt_us as f64 * 100.0).clamp(0.0, 100.0);
        Some(pct as f32)
    }
}

fn is_intel(device: &Path) -> bool {
    let Ok(uevent) = fs::read_to_string(device.join("uevent")) else {
        return false;
    };
    uevent.lines().any(|l| {
        let l = l.trim();
        l == "DRIVER=i915" || l == "DRIVER=xe"
    })
}

fn find_render_engine(card: &Path) -> Option<PathBuf> {
    let engine_root = card.join("engine");
    let entries = fs::read_dir(&engine_root).ok()?;
    let mut candidates: Vec<_> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("busy").is_file())
        .collect();
    candidates.sort();
    // Prefer rcs0 (render); otherwise first available engine.
    candidates
        .iter()
        .find(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == "rcs0")
                .unwrap_or(false)
        })
        .cloned()
        .or_else(|| candidates.into_iter().next())
}

fn read_intel_name(device: &Path) -> String {
    let Ok(uevent) = fs::read_to_string(device.join("uevent")) else {
        return "Intel GPU".to_string();
    };
    let pci_id = uevent
        .lines()
        .find_map(|l| l.strip_prefix("PCI_ID="))
        .unwrap_or("");
    if pci_id.is_empty() {
        "Intel GPU".to_string()
    } else {
        format!("Intel GPU ({pci_id})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sysfs/intel")
    }

    #[test]
    fn probe_finds_intel_card_in_fixture() {
        let backend = IntelSysfs::probe_in(&fixture_root()).expect("should find card0");

        assert!(backend.name().starts_with("Intel GPU"));
        assert!(backend.name().contains("8086:9A49"));
    }

    #[test]
    fn first_sample_returns_none_then_subsequent_returns_value() {
        let mut backend = IntelSysfs::probe_in(&fixture_root()).unwrap();

        let first = backend.sample();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = backend.sample();

        assert!(first.utilization_pct.is_none());
        // Fixture busy is constant 0 → delta is 0% on second read.
        assert_eq!(second.utilization_pct, Some(0.0));
    }
}
