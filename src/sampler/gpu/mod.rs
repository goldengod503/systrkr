//! GPU sampling abstraction. Backends are picked at startup by `probe()`.

pub mod none;
pub mod amd;
pub mod intel;
pub mod procs;

#[cfg(feature = "nvidia")]
pub mod nvml;

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
}

/// Auto-detect available GPU backends in priority order:
/// NVIDIA (NVML) → AMD sysfs → Intel sysfs → no-op fallback.
pub fn probe() -> Box<dyn GpuBackend> {
    #[cfg(feature = "nvidia")]
    if let Some(b) = nvml::Nvml::probe() {
        return Box::new(b);
    }
    if let Some(b) = amd::AmdSysfs::probe() {
        return Box::new(b);
    }
    if let Some(b) = intel::IntelSysfs::probe() {
        return Box::new(b);
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
        if let Ok(lib) = nvml_wrapper::Nvml::init() {
            if let Ok(count) = lib.device_count() {
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
    }

    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        for entry in entries.flatten() {
            let path = entry.path();
            let card_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            if !(card_name.starts_with("card") && !card_name.contains('-')) {
                continue;
            }
            let device = path.join("device");
            let pdev = std::fs::read_link(&device)
                .ok()
                .and_then(|p| p.file_name().and_then(|n| n.to_str().map(|s| s.to_string())))
                .unwrap_or_default();
            if out.iter().any(|g| g.pdev == pdev) {
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
