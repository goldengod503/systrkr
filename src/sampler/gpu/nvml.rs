#![cfg(feature = "nvidia")]

use nvml_wrapper::Nvml as NvmlLib;
use tracing::warn;

use super::{GpuBackend, GpuSample};

pub struct Nvml {
    lib: NvmlLib,
    name: String,
    pdev: String,
    /// `true` once we've already logged a sample failure for this backend.
    sample_warned: bool,
    index: u32,
}

impl Nvml {
    pub fn probe_index(idx: u32) -> Option<Self> {
        let lib = match NvmlLib::init() {
            Ok(l) => l,
            Err(e) => {
                warn!(error = %e, "NVML init failed; skipping NVIDIA backend");
                return None;
            }
        };
        let device = lib.device_by_index(idx).ok()?;
        let name = device
            .name()
            .unwrap_or_else(|_| format!("NVIDIA GPU {idx}"));
        let pdev = device
            .pci_info()
            .ok()
            .map(|p| p.bus_id.to_lowercase())
            .unwrap_or_default();
        Some(Self {
            lib,
            name,
            pdev,
            sample_warned: false,
            index: idx,
        })
    }
}

impl GpuBackend for Nvml {
    fn name(&self) -> &str {
        &self.name
    }

    fn pdev(&self) -> &str {
        &self.pdev
    }

    fn is_nvidia(&self) -> bool {
        true
    }

    fn nvml_index(&self) -> Option<u32> {
        Some(self.index)
    }

    fn sample(&mut self) -> GpuSample {
        let device = match self.lib.device_by_index(self.index) {
            Ok(d) => {
                self.sample_warned = false;
                d
            }
            Err(e) => {
                if !self.sample_warned {
                    warn!(error = %e, index = self.index, "NVML device_by_index failed");
                    self.sample_warned = true;
                }
                return GpuSample::default();
            }
        };

        let utilization_pct = device.utilization_rates().ok().map(|u| u.gpu as f32);
        let mem = device.memory_info().ok();
        let temperature_c = device
            .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
            .ok()
            .map(|t| t as f32);

        GpuSample {
            utilization_pct,
            memory_used_bytes: mem.as_ref().map(|m| m.used),
            memory_total_bytes: mem.as_ref().map(|m| m.total),
            temperature_c,
        }
    }
}
