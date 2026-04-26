use super::{GpuProcSample, GpuProcessBackend};

pub struct FdinfoProcs {
    _pdev: String,
}

impl FdinfoProcs {
    pub fn new(pdev: String) -> Option<Self> {
        Some(Self { _pdev: pdev })
    }
}

impl GpuProcessBackend for FdinfoProcs {
    fn top_n(&mut self, _n: usize) -> Vec<GpuProcSample> {
        Vec::new()
    }
}
