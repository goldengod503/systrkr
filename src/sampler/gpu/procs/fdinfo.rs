//! DRM fdinfo per-process GPU stats. Works for any DRM-scheduler driver
//! (amdgpu, i915, xe, panfrost…). NVIDIA's proprietary driver does not
//! populate these fields — the NVML backend handles NVIDIA.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use super::{GpuProcSample, GpuProcessBackend};

pub struct FdinfoProcs {
    target_pdev: String,
    /// pid -> (last_sample_time, cumulative_busy_ns)
    last: HashMap<u32, (Instant, u64)>,
}

impl FdinfoProcs {
    pub fn new(pdev: String) -> Option<Self> {
        Some(Self {
            target_pdev: pdev,
            last: HashMap::new(),
        })
    }
}

impl GpuProcessBackend for FdinfoProcs {
    fn top_n(&mut self, n: usize) -> Vec<GpuProcSample> {
        let scan = scan_proc_fdinfo(&self.target_pdev);
        let now = Instant::now();
        let mut samples: Vec<GpuProcSample> = scan
            .into_iter()
            .map(|agg| {
                let pct = match self.last.insert(agg.pid, (now, agg.busy_ns)) {
                    Some((prev_t, prev_busy)) => {
                        let dt_ns = now.saturating_duration_since(prev_t).as_nanos() as u64;
                        if dt_ns == 0 {
                            None
                        } else {
                            let dbusy = agg.busy_ns.saturating_sub(prev_busy);
                            Some(((dbusy as f64 / dt_ns as f64) * 100.0).clamp(0.0, 100.0) as f32)
                        }
                    }
                    None => None,
                };
                GpuProcSample {
                    pid: agg.pid,
                    name: read_proc_name(agg.pid).unwrap_or_else(|| format!("pid {}", agg.pid)),
                    memory_bytes: Some(agg.vram_bytes),
                    utilization_pct: pct,
                }
            })
            .collect();

        samples.sort_by(|a, b| {
            let av = a.utilization_pct.unwrap_or(0.0);
            let bv = b.utilization_pct.unwrap_or(0.0);
            bv.partial_cmp(&av).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Garbage-collect against the full scan set (before truncation), not
        // the top-N. Otherwise a process that drops below rank N for one tick
        // loses its delta baseline and emits `None` utilization on re-entry —
        // which sorts it to the bottom and re-evicts it. Self-reinforcing.
        let live: std::collections::HashSet<u32> = samples.iter().map(|s| s.pid).collect();
        self.last.retain(|pid, _| live.contains(pid));

        samples.truncate(n);
        samples
    }
}

#[derive(Debug, Default)]
struct PidAggregate {
    pid: u32,
    /// Sum of drm-engine-* across all fds & clients for this pid.
    busy_ns: u64,
    /// Sum of drm-memory-vram across all fds & clients for this pid.
    vram_bytes: u64,
}

fn scan_proc_fdinfo(target_pdev: &str) -> Vec<PidAggregate> {
    let mut by_pid: HashMap<u32, PidAggregate> = HashMap::new();

    let proc_dir = match fs::read_dir("/proc") {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    for proc_entry in proc_dir.flatten() {
        let name = proc_entry.file_name();
        let name_str = match name.to_str() {
            Some(s) => s,
            None => continue,
        };
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        let fd_dir = proc_entry.path().join("fd");
        let fd_iter = match fs::read_dir(&fd_dir) {
            Ok(it) => it,
            Err(_) => continue,
        };

        for fd_entry in fd_iter.flatten() {
            let target = match fs::read_link(fd_entry.path()) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let tgt_str = target.to_string_lossy();
            if !(tgt_str.starts_with("/dev/dri/card") || tgt_str.starts_with("/dev/dri/render")) {
                continue;
            }

            let fd_name = match fd_entry.file_name().to_str() {
                Some(s) => s.to_string(),
                None => continue,
            };
            let fdinfo_path = proc_entry.path().join("fdinfo").join(&fd_name);
            let content = match fs::read_to_string(&fdinfo_path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let parsed = parse_fdinfo(&content);
            let pdev_match = match &parsed.pdev {
                Some(p) => p == target_pdev,
                None => false,
            };
            if !pdev_match {
                continue;
            }

            let agg = by_pid.entry(pid).or_insert(PidAggregate {
                pid,
                ..Default::default()
            });
            agg.busy_ns = agg.busy_ns.saturating_add(parsed.busy_ns);
            agg.vram_bytes = agg.vram_bytes.saturating_add(parsed.vram_bytes);
        }
    }

    by_pid.into_values().collect()
}

#[derive(Debug, Default)]
struct ParsedFd {
    pdev: Option<String>,
    busy_ns: u64,
    vram_bytes: u64,
}

fn parse_fdinfo(content: &str) -> ParsedFd {
    let mut out = ParsedFd::default();
    for line in content.lines() {
        let (key, value) = match line.split_once(':') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };
        match key {
            "drm-pdev" => out.pdev = Some(value.to_string()),
            k if k.starts_with("drm-engine-") => {
                if let Some(ns) = value.strip_suffix(" ns").and_then(|s| s.parse::<u64>().ok()) {
                    out.busy_ns = out.busy_ns.saturating_add(ns);
                }
            }
            "drm-memory-vram" => {
                out.vram_bytes = parse_kib(value).unwrap_or(0);
            }
            _ => {}
        }
    }
    out
}

fn parse_kib(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    let kib = trimmed.strip_suffix(" KiB").or_else(|| trimmed.strip_suffix(" kB"))?;
    kib.trim().parse::<u64>().ok().map(|k| k * 1024)
}

fn read_proc_name(pid: u32) -> Option<String> {
    fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/fdinfo")
            .join(name)
    }

    #[test]
    fn parser_extracts_pdev_engines_and_vram() {
        let content = std::fs::read_to_string(fixture_path("sample-amd.txt")).unwrap();

        let parsed = parse_fdinfo(&content);

        assert_eq!(parsed.pdev.as_deref(), Some("0000:03:00.0"));
        // gfx (1.5e9) + compute (3e8) + render (0) = 1.8e9 ns
        assert_eq!(parsed.busy_ns, 1_800_000_000);
        // 524288 KiB = 512 MiB
        assert_eq!(parsed.vram_bytes, 524_288 * 1024);
    }

    #[test]
    fn parser_ignores_non_drm_lines() {
        let parsed = parse_fdinfo("pos:\t0\nflags:\t02100002\nino:\t9\n");

        assert!(parsed.pdev.is_none());
        assert_eq!(parsed.busy_ns, 0);
        assert_eq!(parsed.vram_bytes, 0);
    }

    #[test]
    fn parse_kib_handles_kib_and_kb_suffixes() {
        assert_eq!(parse_kib("1024 KiB"), Some(1_048_576));
        assert_eq!(parse_kib("2048 kB"), Some(2_097_152));
        assert_eq!(parse_kib("oops"), None);
    }
}
