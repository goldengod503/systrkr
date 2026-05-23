# GS-02 — Sampler architectural-analysis follow-through

**Date:** 2026-05-23
**Branch:** `main`
**Verified-SHA bump:** `53423ae` → `456bee7` in `docs/ARCHITECTURE.md` + `CLAUDE.md`

## Summary

Ran `robot:architectural-analysis` against `src/sampler/` and executed every actionable recommendation across five commits. Closes silent wrong-data paths (`ConfigUpdated` ignoring `gpu_index` changes, `FdinfoProcs` GC self-evicting boundary processes, `NvmlProcs` hard-coding device 0), restores intended UX strings on no-GPU systems, extends the NET interface filter to exclude container/VPN virtual interfaces, dedupes three sites that were already drifting, and adds `tracing::debug!` instrumentation that feeds the long-standing "sync sampling on UI thread" reopen trigger. ARCHITECTURE.md updated with two new Intentional decisions and one Known-deferred refinement.

## Scope

**Files Modified**
- `src/app.rs` — `Message::ConfigUpdated` now rebuilds `Sampler` when `gpu_index` changes.
- `src/sampler/mod.rs` — `Sampler::new` passes `gpu_backend.nvml_index()` to `gpu::procs::probe`.
- `src/sampler/net.rs` — `is_virtual_iface` helper extends the filter to `veth*`, `virbr*`, `tun*`, `tap*`, `podman*`, `wg*`, `cni*` in addition to `lo`, `docker*`, `br-*`.
- `src/sampler/gpu/mod.rs` — `GpuBackend` trait gains `nvml_index() -> Option<u32>` (default `None`). Adds shared `is_card_dir(name)` and `resolve_card_by_pdev(pdev)` helpers used by AMD, Intel, and `enumerate`. Removes dead `pub fn probe()`. Type-annotates the closure in `enumerate` so `cargo check --no-default-features` compiles (was broken at baseline despite README claim).
- `src/sampler/gpu/amd.rs` — `probe_pdev` delegates to `resolve_card_by_pdev`; `iter_cards` uses `is_card_dir`. Removes dead `pub fn probe()`.
- `src/sampler/gpu/intel.rs` — same delegation/dedup; removes dead `pub fn probe()`.
- `src/sampler/gpu/nvml.rs` — overrides `nvml_index()` to return `Some(self.index)`; resets `sample_warned = false` on successful `device_by_index`. Removes dead `pub fn probe()`.
- `src/sampler/gpu/procs/mod.rs` — `probe` signature extended with `nvml_index: Option<u32>`; threads index into `NvmlProcs::new(idx)`. Adds shared `read_proc_name(pid)`.
- `src/sampler/gpu/procs/nvml.rs` — `NvmlProcs::new(idx)` stores the index; `top_n` uses `self.index` instead of hard-coded `0`. Drops the local `read_proc_name`.
- `src/sampler/gpu/procs/fdinfo.rs` — `new` rejects empty pdev; GC step moved before truncation so it operates on the full live scan, not the displayed top-N; `as_nanos() as u64` replaced with saturating `u64::try_from(...).unwrap_or(u64::MAX)`; drops local `read_proc_name` in favour of the shared one; adds `tracing::debug!` capturing `elapsed_us` + `scanned` count; drops unused `std::path::PathBuf` import.
- `docs/ARCHITECTURE.md` — clarifies `Send` supertrait is forward-compat, documents dual-`NvmlLib::init()` as deliberate, updates the sync-sampling Known-deferred entry to reference the new tracing line, adds the 2026-05-23 entry under Recent changes, bumps verified SHA.
- `CLAUDE.md` — matching verified-SHA bump.

**Files Created**
- `docs/releases/2026-05-23_GS-02_sampler-analysis-followthrough.md` — this file.

**Commits (oldest → newest)**
- `ef351c9` — `fix(app): rebuild Sampler when ConfigUpdated changes gpu_index` (A3)
- `8c76727` — `fix(sampler): GC FdinfoProcs.last against full scan set, not top-N` (A4)
- `ad524b0` — `fix(sampler): GPU-process correctness, NVML warn reset, NET filter` (A1+A2+A5+A6)
- `456bee7` — `refactor(sampler/gpu): extract shared helpers, drop dead probe() fns` (A7+A8+A9 + drive-bys)
- `2a1eeec` — `docs: refresh ARCHITECTURE.md against 456bee7` (A10 + tracing instrumentation)

## Behavioral Impact

- **GPU selection from outside the applet now applies.** `cosmic-settings`, `dconf`, or a second instance writing `gpu_index` retargets the sampler instead of silently desyncing.
- **AMD/Intel per-process GPU utilization stops dashing out for boundary processes.** Any process oscillating across the rank-5 boundary now retains its delta baseline and shows numeric utilization on re-entry.
- **Multi-NVIDIA workstations selecting card 1+ now see card 1's processes.** Previously the per-process list always showed card 0.
- **No-GPU systems show the intended "Per-process GPU not supported" message** instead of `(idle)`.
- **NVML failure logging resumes after driver-reload episodes** (Optimus power-gating, DKMS update). Previously the warn was suppressed for the lifetime of the applet after the first failure.
- **NET sparkline reflects physical traffic on Docker/libvirt/VPN dev machines.** `veth*`, `virbr*`, `tun*`, `tap*`, `podman*`, `wg*`, `cni*` no longer count.
- **`cargo build --no-default-features` works again** (AMD-only build path from the README).
- `tracing::debug!` in `FdinfoProcs::top_n` is new but only logs at `RUST_LOG=galaxy_systrkr=debug` — silent at default log level.

## Test Plan

- Build: `cargo check --all-features` and `cargo check --no-default-features` both pass with zero warnings.
- Tests: `cargo test --all-features` — **39 passed, 0 failed**. Existing test coverage for `RingBuf`, `CpuSampler`, `NetSampler`, `IntelSysfs`, `AmdSysfs`, `NoGpu`, `Sampler::tick`, and fdinfo parsing all still pass.
- Manual: not run — analytical follow-through, no UX-shape changes. Run with `RUST_LOG=galaxy_systrkr=debug` to verify the new tracing line emits on every tick.
- Clippy: `cargo clippy --all-features -- -D warnings` was red at baseline with 16 pre-existing errors; not addressed in this stream.

## Docs Updated

- `docs/ARCHITECTURE.md` — two new Intentional decisions (`Send` clarification, dual `NvmlLib::init()`), one Known-deferred refinement (sync-sampling instrumentation), Recent changes entry, SHA bump.
- `CLAUDE.md` — matching SHA bump.
- `docs/release-ledger.md` — GS-02 row added.

## Rollback Plan

`git revert 2a1eeec 456bee7 ad524b0 8c76727 ef351c9` rolls back the whole stream in reverse order. The doc-refresh and SHA bump are in `2a1eeec` and revert cleanly. No schema, no config-version bump, no persistence migration — all changes are pure code + doc.

For per-commit rollback (recommended if a regression is isolated): revert the single offending SHA. Each commit is self-contained; the architectural-decisions doc edits in `2a1eeec` reference behavior that still holds with any of the five reverted.

## Open Questions / Decisions

None new. The `Send` supertrait clarification and the dual-`NvmlLib::init()` rationale are recorded as Intentional decisions in `docs/ARCHITECTURE.md` so future analytical runs classify any "remove this" finding as `Contradicts` (and require a counter-argument).

Pre-existing items not touched by this stream:
- 16 `cargo clippy --all-features -- -D warnings` errors at baseline — separate cleanup, out of scope.
- The Known-deferred items in ARCHITECTURE.md that remain Known-deferred: GPU enumeration walk on Settings open, network/disk aggregation across all interfaces (the architect's A6 extension here partially addresses NET but per-interface UX is still deferred), `OllamaProber::new().expect()`, structured `Result`-typed sampler errors.
