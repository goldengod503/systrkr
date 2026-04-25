use super::{GpuBackend, GpuSample};

pub struct AmdSysfs;

impl AmdSysfs {
    pub fn probe() -> Option<Self> {
        None
    }
}

impl GpuBackend for AmdSysfs {
    fn name(&self) -> &str { "AMD GPU" }
    fn sample(&mut self) -> GpuSample { GpuSample::default() }
}
