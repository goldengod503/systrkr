#![cfg(feature = "nvidia")]

use super::{GpuProcSample, GpuProcessBackend};

pub struct NvmlProcs;

impl NvmlProcs {
    pub fn new() -> Option<Self> {
        None // implemented in Task 5
    }
}

impl GpuProcessBackend for NvmlProcs {
    fn top_n(&mut self, _n: usize) -> Vec<GpuProcSample> {
        Vec::new()
    }
}
