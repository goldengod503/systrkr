use std::fs;
use std::path::{Path, PathBuf};

use super::{GpuBackend, GpuSample};

pub struct AmdSysfs {
    card_path: PathBuf,
    name: String,
    pdev: String,
}

impl AmdSysfs {
    pub fn probe_in(drm_root: &Path) -> Option<Self> {
        for card in iter_cards(drm_root)? {
            if let Some(backend) = Self::probe_specific(&card) {
                return Some(backend);
            }
        }
        None
    }

    pub(crate) fn probe_specific(card: &Path) -> Option<Self> {
        let device = card.join("device");
        if !is_amdgpu(&device) {
            return None;
        }
        if !device.join("gpu_busy_percent").is_file() {
            return None;
        }
        let name = read_amdgpu_name(&device).unwrap_or_else(|| "AMD GPU".to_string());
        let pdev = fs::read_link(&device)
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
            .unwrap_or_default();
        Some(Self {
            card_path: card.to_path_buf(),
            name,
            pdev,
        })
    }

    pub fn probe_pdev(pdev: &str) -> Option<Self> {
        super::resolve_card_by_pdev(pdev).and_then(|card| Self::probe_specific(&card))
    }

    fn device(&self) -> PathBuf {
        self.card_path.join("device")
    }
}

impl GpuBackend for AmdSysfs {
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
        let dev = self.device();
        GpuSample {
            utilization_pct: read_trim(&dev.join("gpu_busy_percent"))
                .and_then(|s| s.parse::<f32>().ok()),
            memory_total_bytes: read_trim(&dev.join("mem_info_vram_total"))
                .and_then(|s| s.parse::<u64>().ok()),
            memory_used_bytes: read_trim(&dev.join("mem_info_vram_used"))
                .and_then(|s| s.parse::<u64>().ok()),
            temperature_c: read_amdgpu_temp(&dev),
        }
    }
}

fn iter_cards(drm_root: &Path) -> Option<impl Iterator<Item = PathBuf>> {
    let entries = fs::read_dir(drm_root).ok()?;
    Some(entries.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .map(super::is_card_dir)
            .unwrap_or(false)
    }))
}

fn is_amdgpu(device: &Path) -> bool {
    let Some(uevent) = read_trim(&device.join("uevent")) else {
        return false;
    };
    uevent.lines().any(|l| l.trim() == "DRIVER=amdgpu")
}

fn read_amdgpu_name(device: &Path) -> Option<String> {
    // The kernel doesn't expose a friendly model string; PCI_ID is the best we have.
    let uevent = read_trim(&device.join("uevent"))?;
    let pci_id = uevent
        .lines()
        .find_map(|l| l.strip_prefix("PCI_ID="))?
        .to_string();
    Some(format!("AMD GPU ({pci_id})"))
}

fn read_amdgpu_temp(device: &Path) -> Option<f32> {
    let hwmon_root = device.join("hwmon");
    let entries = fs::read_dir(&hwmon_root).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path().join("temp1_input");
        if let Some(raw) = read_trim(&candidate) {
            if let Ok(milli) = raw.parse::<i32>() {
                return Some(milli as f32 / 1000.0);
            }
        }
    }
    None
}

fn read_trim(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sysfs/amd")
    }

    #[test]
    fn probe_finds_amd_card_in_fixture() {
        let backend = AmdSysfs::probe_in(&fixture_root()).expect("should find card0");

        assert!(backend.name().starts_with("AMD GPU"));
        assert!(backend.name().contains("1002:73BF"));
    }

    #[test]
    fn sample_parses_all_fields() {
        let mut backend = AmdSysfs::probe_in(&fixture_root()).unwrap();

        let sample = backend.sample();

        assert_eq!(sample.utilization_pct, Some(42.0));
        assert_eq!(sample.memory_total_bytes, Some(17_163_091_968));
        assert_eq!(sample.memory_used_bytes, Some(2_147_483_648));
        assert_eq!(sample.temperature_c, Some(55.0));
    }

    #[test]
    fn probe_returns_none_for_empty_root() {
        let tmp = tempfile::tempdir().unwrap();

        let result = AmdSysfs::probe_in(tmp.path());

        assert!(result.is_none());
    }
}
