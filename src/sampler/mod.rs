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
