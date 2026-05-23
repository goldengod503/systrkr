//! Per-process GPU stats. Backend choice depends on the GPU vendor:
//! NVIDIA → NVML; AMD/Intel → DRM fdinfo (/proc/*/fdinfo/).

pub mod fdinfo;

#[cfg(feature = "nvidia")]
pub mod nvml;

#[derive(Debug, Clone, Default)]
pub struct GpuProcSample {
    pub pid: u32,
    pub name: String,
    /// Bytes of GPU memory used by this process. None when the backend cannot
    /// report it (e.g., fdinfo on a card that doesn't expose memory keys).
    pub memory_bytes: Option<u64>,
    /// 0..=100 utilization on the render engine. None when unavailable.
    pub utilization_pct: Option<f32>,
}

pub trait GpuProcessBackend: Send {
    /// Return top `n` processes by GPU memory (NVML) or GPU engine busy% (fdinfo).
    fn top_n(&mut self, n: usize) -> Vec<GpuProcSample>;
}

/// Picks the right backend for the given GPU pdev (PCI address like
/// "0000:2b:00.0"). NVIDIA pdevs go to NVML and need the device index;
/// everything else to fdinfo. Returns None if neither backend can serve
/// the pdev — e.g., when the selected GPU is `NoGpu` (empty pdev).
pub fn probe(
    pdev: &str,
    is_nvidia: bool,
    nvml_index: Option<u32>,
) -> Option<Box<dyn GpuProcessBackend>> {
    #[cfg(feature = "nvidia")]
    if is_nvidia {
        if let Some(idx) = nvml_index {
            if let Some(b) = nvml::NvmlProcs::new(idx) {
                return Some(Box::new(b));
            }
        }
    }
    let _ = (is_nvidia, nvml_index); // silence warning when nvidia feature is off
    if let Some(b) = fdinfo::FdinfoProcs::new(pdev.to_string()) {
        return Some(Box::new(b));
    }
    None
}
