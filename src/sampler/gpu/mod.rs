//! GPU sampling abstraction. Backends are picked at startup by `probe_index()`.

pub mod none;
pub mod amd;
pub mod intel;
pub mod procs;

#[cfg(feature = "nvidia")]
pub mod nvml;

use std::path::PathBuf;

use super::GpuSample;

pub trait GpuBackend: Send {
    /// Human-readable model name shown in the popup.
    fn name(&self) -> &str;

    /// Read one sample. Individual fields may be `None` on partial failure.
    fn sample(&mut self) -> GpuSample;

    /// PCI device address like "0000:2b:00.0" or empty if not applicable.
    fn pdev(&self) -> &str;

    /// True if this is the NVIDIA backend (driver doesn't populate fdinfo).
    fn is_nvidia(&self) -> bool;

    /// NVML device index for NVIDIA backends so per-process readers can
    /// target the same card. `None` for every non-NVIDIA backend.
    fn nvml_index(&self) -> Option<u32> {
        None
    }
}

/// Select GPU backend for the given index using `enumerate()` to identify the card.
/// Falls back to index 0 if the requested index is out of range, and to the no-op
/// backend if no GPU is found at all.
pub fn probe_index(index: usize) -> Box<dyn GpuBackend> {
    let infos = enumerate();
    let target = infos.get(index).or_else(|| infos.first());

    if let Some(info) = target {
        #[cfg(feature = "nvidia")]
        if info.is_nvidia {
            // Count only the NVIDIA entries that appear before this one in the
            // enumeration list to get the NVML device index.
            let nvml_idx = infos[..info.index]
                .iter()
                .filter(|g| g.is_nvidia)
                .count() as u32;
            if let Some(b) = nvml::Nvml::probe_index(nvml_idx) {
                return Box::new(b);
            }
        }
        if let Some(b) = amd::AmdSysfs::probe_pdev(&info.pdev) {
            return Box::new(b);
        }
        if let Some(b) = intel::IntelSysfs::probe_pdev(&info.pdev) {
            return Box::new(b);
        }
    }
    Box::new(none::NoGpu::new())
}

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub index: usize,
    pub name: String,
    pub pdev: String,
    pub is_nvidia: bool,
}

pub fn enumerate() -> Vec<GpuInfo> {
    let mut out = Vec::new();

    #[cfg(feature = "nvidia")]
    {
        if let Ok(lib) = nvml_wrapper::Nvml::init()
            && let Ok(count) = lib.device_count()
        {
            for i in 0..count {
                if let Ok(d) = lib.device_by_index(i) {
                    let name = d.name().unwrap_or_else(|_| format!("NVIDIA GPU {i}"));
                    let pdev = d
                        .pci_info()
                        .ok()
                        .map(|p| p.bus_id.to_lowercase())
                        .unwrap_or_default();
                    out.push(GpuInfo {
                        index: out.len(),
                        name: format!("NVIDIA {name}"),
                        pdev,
                        is_nvidia: true,
                    });
                }
            }
        }
    }

    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path();
            let card_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if !is_card_dir(&card_name) {
                continue;
            }
            let device = path.join("device");
            let pdev = std::fs::read_link(&device)
                .ok()
                .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
                .unwrap_or_default();
            if out.iter().any(|g: &GpuInfo| g.pdev == pdev) {
                continue;
            }
            let uevent = std::fs::read_to_string(device.join("uevent")).unwrap_or_default();
            let driver = uevent
                .lines()
                .find_map(|l| l.strip_prefix("DRIVER="))
                .unwrap_or("unknown")
                .to_string();
            out.push(GpuInfo {
                index: out.len(),
                name: format!("{driver} ({card_name})"),
                pdev,
                is_nvidia: false,
            });
        }
    }

    out
}

/// True for `/sys/class/drm/cardN` directories. Excludes the connector
/// subdirectories like `card0-DP-1` which contain a hyphen.
pub(crate) fn is_card_dir(name: &str) -> bool {
    name.starts_with("card") && !name.contains('-')
}

/// Find the `/sys/class/drm/cardN` directory whose `device` symlink
/// resolves to `pdev`. Returns `None` if no card matches or `pdev` is
/// empty (the `NoGpu` sentinel).
pub(crate) fn resolve_card_by_pdev(pdev: &str) -> Option<PathBuf> {
    if pdev.is_empty() {
        return None;
    }
    let entries = std::fs::read_dir("/sys/class/drm").ok()?;
    for entry in entries.flatten() {
        let card = entry.path();
        let name = card
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !is_card_dir(name) {
            continue;
        }
        let device = card.join("device");
        let card_pdev = std::fs::read_link(&device)
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
            .unwrap_or_default();
        if card_pdev == pdev {
            return Some(card);
        }
    }
    None
}
