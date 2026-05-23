use nvml_wrapper::Nvml as NvmlLib;
use tracing::warn;

use super::{read_proc_name, GpuProcSample, GpuProcessBackend};

pub struct NvmlProcs {
    lib: NvmlLib,
    index: u32,
}

impl NvmlProcs {
    pub fn new(index: u32) -> Option<Self> {
        match NvmlLib::init() {
            Ok(lib) => Some(Self { lib, index }),
            Err(e) => {
                warn!(error = %e, "NVML init for per-process failed");
                None
            }
        }
    }
}

impl GpuProcessBackend for NvmlProcs {
    fn top_n(&mut self, n: usize) -> Vec<GpuProcSample> {
        let device = match self.lib.device_by_index(self.index) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };

        let mut combined: Vec<GpuProcSample> = Vec::new();

        if let Ok(procs) = device.running_compute_processes() {
            for p in procs {
                let mem = match p.used_gpu_memory {
                    nvml_wrapper::enums::device::UsedGpuMemory::Used(b) => b,
                    nvml_wrapper::enums::device::UsedGpuMemory::Unavailable => 0,
                };
                push_proc(&mut combined, p.pid, mem);
            }
        }
        if let Ok(procs) = device.running_graphics_processes() {
            for p in procs {
                if combined.iter().any(|x| x.pid == p.pid) {
                    continue;
                }
                let mem = match p.used_gpu_memory {
                    nvml_wrapper::enums::device::UsedGpuMemory::Used(b) => b,
                    nvml_wrapper::enums::device::UsedGpuMemory::Unavailable => 0,
                };
                push_proc(&mut combined, p.pid, mem);
            }
        }

        combined.sort_by_key(|p| std::cmp::Reverse(p.memory_bytes.unwrap_or(0)));
        combined.truncate(n);
        combined
    }
}

fn push_proc(list: &mut Vec<GpuProcSample>, pid: u32, mem: u64) {
    let name = read_proc_name(pid).unwrap_or_else(|| format!("pid {pid}"));
    list.push(GpuProcSample {
        pid,
        name,
        memory_bytes: Some(mem),
        utilization_pct: None,
    });
}
