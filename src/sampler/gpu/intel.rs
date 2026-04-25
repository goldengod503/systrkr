use super::{GpuBackend, GpuSample};

pub struct IntelSysfs;

impl IntelSysfs {
    pub fn probe() -> Option<Self> {
        None
    }
}

impl GpuBackend for IntelSysfs {
    fn name(&self) -> &str { "Intel GPU" }
    fn sample(&mut self) -> GpuSample { GpuSample::default() }
}
