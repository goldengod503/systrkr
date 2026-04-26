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
