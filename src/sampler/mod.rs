//! Sampling layer: combines CPU and GPU readings into a single Sample.

pub mod cpu;
pub mod gpu;

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub cpu: CpuSample,
    pub gpu: GpuSample,
}

#[derive(Clone, Debug, Default)]
pub struct CpuSample {
    pub utilization_pct: Option<f32>,
    pub temperature_c: Option<f32>,
    pub model: Option<String>,
    pub ram_used_bytes: Option<u64>,
    pub ram_total_bytes: Option<u64>,
    pub swap_used_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
}

#[derive(Clone, Debug, Default)]
pub struct GpuSample {
    pub utilization_pct: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub temperature_c: Option<f32>,
}

use cpu::CpuSampler;
use gpu::GpuBackend;

pub struct Sampler {
    cpu: CpuSampler,
    gpu: Box<dyn GpuBackend>,
}

impl Sampler {
    pub fn new() -> Self {
        Self {
            cpu: CpuSampler::new(),
            gpu: gpu::probe(),
        }
    }

    pub fn gpu_name(&self) -> &str {
        self.gpu.name()
    }

    pub fn tick(&mut self) -> Sample {
        Sample {
            cpu: self.cpu.tick(),
            gpu: self.gpu.sample(),
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod aggregator_tests {
    use super::*;

    #[test]
    fn two_consecutive_ticks_dont_panic() {
        let mut s = Sampler::new();

        let _ = s.tick();
        let _ = s.tick();
    }
}
