#![cfg(feature = "nvidia")]

use super::{GpuBackend, GpuSample};

pub struct Nvml;

impl Nvml {
    pub fn probe() -> Option<Self> {
        None
    }
}

impl GpuBackend for Nvml {
    fn name(&self) -> &str { "NVIDIA GPU" }
    fn sample(&mut self) -> GpuSample { GpuSample::default() }
}
