use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::{GpuBackend, GpuSample};

pub struct IntelSysfs {
    card_path: PathBuf,
    name: String,
    last: Option<(Instant, u64)>,
}

impl IntelSysfs {
    pub fn probe() -> Option<Self> {
        Self::probe_in(Path::new("/sys/class/drm"))
    }

    pub fn probe_in(drm_root: &Path) -> Option<Self> {
        let entries = fs::read_dir(drm_root).ok()?;
        for entry in entries.flatten() {
            let card = entry.path();
            if !card
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("card") && !n.contains('-'))
                .unwrap_or(false)
            {
                continue;
            }
            let device = card.join("device");
            if !is_intel(&device) {
                continue;
            }
            if find_render_engine(&card).is_none() {
                continue;
            }
            let name = read_intel_name(&device);
            return Some(Self {
                card_path: card,
                name,
                last: None,
            });
        }
        None
    }
}

impl GpuBackend for IntelSysfs {
    fn name(&self) -> &str {
        &self.name
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
        let dt_us = now.saturating_duration_since(prev_t).as_micros() as u64;
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
