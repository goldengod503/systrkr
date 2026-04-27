//! Sampling layer: combines CPU and GPU readings into a single Sample.

pub mod cpu;
pub mod gpu;
pub mod net;
pub mod procs;

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub cpu: CpuSample,
    pub gpu: GpuSample,
    pub net: net::NetSample,
    pub top_cpu_procs: Vec<crate::sampler::procs::ProcSample>,
    pub top_gpu_procs: Vec<crate::sampler::gpu::procs::GpuProcSample>,
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
    gpu_backend: Box<dyn GpuBackend>,
    proc_sampler: procs::ProcSampler,
    gpu_proc_backend: Option<Box<dyn gpu::procs::GpuProcessBackend>>,
    net: net::NetSampler,
}

impl Sampler {
    pub fn new(cfg: &crate::config::SystrkrConfig) -> Self {
        let gpu_backend = gpu::probe_index(cfg.gpu_index);
        let gpu_proc_backend = gpu::procs::probe(gpu_backend.pdev(), gpu_backend.is_nvidia());
        Self {
            cpu: CpuSampler::new(),
            gpu_backend,
            proc_sampler: procs::ProcSampler::new(),
            gpu_proc_backend,
            net: net::NetSampler::new(),
        }
    }

    pub fn gpu_name(&self) -> &str {
        self.gpu_backend.name()
    }

    pub fn gpu_proc_backend_available(&self) -> bool {
        self.gpu_proc_backend.is_some()
    }

    pub fn tick(&mut self) -> Sample {
        Sample {
            cpu: self.cpu.tick(),
            gpu: self.gpu_backend.sample(),
            net: self.net.tick(),
            top_cpu_procs: self.proc_sampler.top_n(5),
            top_gpu_procs: self
                .gpu_proc_backend
                .as_mut()
                .map(|b| b.top_n(5))
                .unwrap_or_default(),
        }
    }
}

impl Default for Sampler {
    fn default() -> Self {
        Self::new(&crate::config::SystrkrConfig::default())
    }
}

#[cfg(test)]
mod aggregator_tests {
    use super::*;

    #[test]
    fn two_consecutive_ticks_dont_panic() {
        let mut s = Sampler::new(&crate::config::SystrkrConfig::default());

        let _ = s.tick();
        let _ = s.tick();
    }
}
