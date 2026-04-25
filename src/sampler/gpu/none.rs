use super::{GpuBackend, GpuSample};

pub struct NoGpu {
    name: String,
}

impl NoGpu {
    pub fn new() -> Self {
        Self {
            name: "No GPU detected".to_string(),
        }
    }
}

impl Default for NoGpu {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuBackend for NoGpu {
    fn name(&self) -> &str {
        &self.name
    }

    fn sample(&mut self) -> GpuSample {
        GpuSample::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_gpu_sample_is_all_none() {
        let mut g = NoGpu::new();

        let sample = g.sample();

        assert!(sample.utilization_pct.is_none());
        assert!(sample.memory_used_bytes.is_none());
        assert!(sample.memory_total_bytes.is_none());
        assert!(sample.temperature_c.is_none());
    }
}
