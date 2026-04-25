//! GPU sampling abstraction. Backends are picked at startup by `probe()`.

pub mod none;
pub mod amd;
pub mod intel;

#[cfg(feature = "nvidia")]
pub mod nvml;

use super::GpuSample;

pub trait GpuBackend: Send {
    /// Human-readable model name shown in the popup.
    fn name(&self) -> &str;

    /// Read one sample. Individual fields may be `None` on partial failure.
    fn sample(&mut self) -> GpuSample;
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
