# galaxy-systrkr Architecture

Living reference for the codebase's structure, the deliberate choices
that shape it, and the issues we know about and consciously deferred.
The point of this doc is to give future architectural reviews a
baseline to compare against — findings that contradict a stated
decision here must argue against the doc rather than against a vacuum.

**Last verified against commit:** 53423ae (2026-05-23, initial draft from /robot:project-documentation first-run — please review and edit before relying on it)

## Current structure

galaxy-systrkr is a single-binary native [COSMIC](https://system76.com/cosmic)
panel applet written in Rust 2024. It uses `libcosmic` (Iced under the
hood) for the GUI and runs an Elm-style update/view loop driven by a
periodic `Tick` subscription.

```
src/
├── main.rs              entry point: tracing init → cosmic::applet::run::<App>
├── lib.rs               re-exports the modules below
├── app.rs               Application impl: state, Message, update, view, subscription
├── popup.rs             popup view composition (CPU/GPU sections, proc lists, ollama)
├── settings.rs          in-popup settings UI (refresh, history, thresholds, toggles)
├── config.rs            SystrkrConfig persisted via cosmic-config (CONFIG_VERSION=2)
├── history.rs           RingBuf<T>: fixed-cap ring with runtime resize
├── widgets/
│   ├── sparkline.rs     iced canvas Program; Scale::Percent | Scale::AutoMax
│   └── proc_list.rs     top-N process row rendering
├── sampler/
│   ├── mod.rs           Sampler aggregator → Sample { cpu, gpu, net, disk, top_*_procs }
│   ├── cpu.rs           sysinfo-backed CPU + RAM + load avg + thermals
│   ├── net.rs           sysinfo Networks delta → bytes/sec
│   ├── disk.rs          /proc/diskstats delta → read/write bytes/sec
│   ├── procs.rs         sysinfo top-N processes by CPU%
│   └── gpu/
│       ├── mod.rs       GpuBackend trait + probe_index() + enumerate()
│       ├── none.rs      NoGpu fallback when nothing is detected
│       ├── amd.rs       AMD sysfs reader
│       ├── intel.rs     Intel sysfs reader
│       ├── nvml.rs      NVIDIA NVML reader (cfg(feature = "nvidia"))
│       └── procs/
│           ├── mod.rs   GpuProcessBackend trait + probe(pdev, is_nvidia)
│           ├── fdinfo.rs  DRM fdinfo reader (AMD, Intel)
│           └── nvml.rs    NVML per-process reader (cfg(feature = "nvidia"))
└── ollama/
    ├── mod.rs           re-exports
    ├── probe.rs         async OllamaProber: GET /api/version, /api/ps via reqwest
    ├── state.rs         OllamaSnapshot, OllamaStatus, OllamaModel
    └── view.rs          popup section renderer
tests/fixtures/          sysfs + fdinfo fixtures consumed by unit tests in modules
```

**Dependency direction (the rules):**

- `main` → `app` only. Nothing else lives at the top level.
- `app` may depend on any module; no module may depend on `app` except
  via the `Message` enum it re-exports.
- `popup`, `settings`, `widgets`, `ollama::view` produce
  `Element<'_, Message>` and are the only modules that touch `cosmic::widget`.
- `sampler::*` is GUI-free. It may depend on `config` for construction
  parameters but never on `app` or `widgets`.
- `sampler::gpu::procs::*` selects its backend based on the *currently
  chosen* GPU's vendor (passed in as `pdev` + `is_nvidia`), not on
  global availability.
- `ollama::probe` is the only async module; everything else is sync and
  runs on the UI thread inside `Sampler::tick()`.

## Intentional decisions

### `GpuBackend` trait with four implementations is not over-abstraction

`src/sampler/gpu/mod.rs:13-25` defines the trait; `none.rs`, `amd.rs`,
`intel.rs`, `nvml.rs` each implement it. The data sources are
completely different (NVML library calls, AMD-specific sysfs files,
Intel-specific sysfs files, no-op), so a single concrete type with
internal branching would be larger and harder to follow. **Rationale:**
the trait boundary matches a real fork in the data path, not a
hypothetical extension point. Rejected alternative: enum over backends
— would require all backend state in every variant, including NVML's
library handle on AMD-only builds.

### `GpuProcessBackend` is a *second* trait, not folded into `GpuBackend`

Per-process GPU stats use different sources from per-GPU stats: NVIDIA
exposes them via NVML, but AMD/Intel expose them via `/proc/*/fdinfo/`
which is independent of `/sys/class/drm`. `src/sampler/gpu/procs/mod.rs:28`
makes the choice based on the *selected* GPU's vendor and pdev, not on
global presence. **Rationale:** a user with both NVIDIA and AMD cards
should see fdinfo procs when they pick the AMD card and NVML procs
when they pick the NVIDIA card. Folding into `GpuBackend` would
conflate "what GPU am I reading from" with "where do per-process
numbers come from."

### `RingBuf<T>` instead of `VecDeque`

`src/history.rs:5-12` is a custom fixed-capacity ring buffer that
supports runtime `resize()` while preserving the most recent samples
(`history.rs:56-77`). `VecDeque` does not preserve "most recent" on
shrink, and `Vec<T>` allocates on every push at capacity. **Rationale:**
the history capacity changes whenever the user adjusts refresh rate or
history seconds in Settings, and we want the visible sparkline to
remain coherent across that change. Test coverage in `history.rs:80-160`
locks the resize semantics down.

### Single `App` struct holds all state, including iced `Cache` handles

`src/app.rs:40-61`. No Redux-style state separation, no per-section
sub-states. Each sparkline gets its own `Cache` because iced's canvas
caches are invalidated explicitly per-canvas. **Rationale:** an Elm-style
applet has one update loop and one state; the cosmic/iced idiom is to
keep the whole picture in `Application::State`. Splitting would require
threading message routing through child components for no behavioral
gain.

### `feature = "nvidia"` is default-on but the crate compiles without it

`Cargo.toml:8-9` and the `cfg(feature = "nvidia")` gates in
`src/sampler/gpu/mod.rs:8-9`, `src/sampler/gpu/procs/mod.rs:6-7`.
**Rationale:** the NVML wrapper links against libnvml at runtime; on
AMD-only systems users must build with `--no-default-features` to
avoid the runtime dependency. The default is "most users have nvidia
or don't notice the dlopen."

### `cosmic-config` schema versioning is explicit, with a `CONFIG_VERSION` constant

`src/config.rs:4` pins `CONFIG_VERSION = 2`; bumped from 1 when Ollama
fields landed. The schema is live-reloaded via
`config_subscription` in `app.rs:427`. **Rationale:** cosmic-config
silently rejects entries that don't match the version, so a missed
bump produces "settings don't persist" symptoms. The constant exists
to make the bump deliberate.

### Ollama probing is opt-in and only subscribes when enabled

`src/app.rs:432-436`. The 5-second `OllamaTick` subscription is only
added to the batch when `config.show_ollama` is true. **Rationale:**
users without ollama installed should pay zero overhead. Reqwest +
tokio are already in the dependency graph, but the network round-trip
should not happen unprompted.

### `Sampler::tick()` runs synchronously on the UI thread

`src/sampler/mod.rs:75-88` is called from `Message::Tick` in `app.rs:113`.
Sysinfo and sysfs reads benchmark well under 1 ms in practice.
**Rationale:** introducing a worker thread would require channels,
backpressure, and would also force `Sample` to be `Send + 'static`. The
current cost is measured, not assumed; reopen if any backend ever
needs more than 10 ms (see Known-deferred issues).

### Top-N hardcoded to 5 for both CPU and GPU process lists

`src/sampler/mod.rs:81-86`. **Rationale:** popup vertical space is the
constraint, not the sampling cost. A configurable N would surface a UI
slider for marginal value; the YAGNI line is drawn here.

## Known-deferred issues

### Synchronous sampling on the UI thread

`Sampler::tick()` blocks the cosmic applet's update loop. Today's
backends are fast; if a future backend (e.g., a Vulkan-based GPU
counter) needs >10 ms per sample the applet will visibly jank at
500 ms tick.

**Reopen when:** any sampler ever logs a `tick()` duration over 10 ms,
or a new backend lands that is expected to be slow.

### GPU enumeration walks `/sys/class/drm` every time Settings opens

`src/sampler/gpu/mod.rs:70-132` is called from `src/settings.rs:37`.
Cheap today (single-digit ms) but the cost grows with attached devices.

**Reopen when:** a multi-GPU workstation reports settings-popup lag,
or the enumeration cost is shown to exceed 5 ms in tracing.

### Network and disk samplers aggregate across all interfaces / devices

`src/sampler/net.rs` sums every interface (verify loopback handling
before per-interface UX work) and `src/sampler/disk.rs` reads
`/proc/diskstats` for all devices and aggregates. There is no
per-interface or per-disk breakdown.

**Reopen when:** a user requests per-interface or per-disk drilldown,
or when loopback traffic visibly inflates the NET sparkline on
someone's machine.

### `OllamaProber::new` panics on reqwest client build failure

`src/ollama/probe.rs:18-24` uses `.expect()` because the default
builder cannot fail in practice. A future feature that lets the user
configure TLS roots, custom CA bundles, or a proxy would need to
return `Result` and surface the error to the settings UI.

**Reopen when:** TLS configuration or proxy support is asked for.

### No structured `Result`-typed error surface from samplers

Samplers return `Option<T>` on failure rather than a tagged error
type. Today this is sufficient because the popup just renders "—" for
missing values. If telemetry on *why* a value is missing ever matters
(e.g., to write a self-diagnostic in Settings), `Option` will need to
become `Result<T, SamplerError>`.

**Reopen when:** a user-visible "Why is GPU data missing?" affordance
becomes a goal.

## Recent architectural changes

- **2026-05-16** — `fix(popup): add 14px gap between popup and panel bar`
  (80b482a). Popup positioner offset bumped by ±14 px in
  `app.rs:162-166`.
- **2026-04-26** — `chore: rename project to galaxy-systrkr` (80067d7).
  Binary name, app ID (`com.goldengod503.GalaxySysTrkr`), cosmic-config
  ID, desktop entry, and icon all renamed in one commit.
- **2026-04-26** — Ollama integration landed across four commits:
  `feat(ollama): snapshot types for probe results` (95273e7) →
  `feat(ollama): async prober for /api/version + /api/ps` (d7f2ae6) →
  `feat(ollama): popup section renderer` (f48d730) →
  `feat(app): wire Ollama prober into subscription and popup` (e531e5e).
  Introduced the `ollama/` module — the only async module — and the
  conditional 5-second `OllamaTick` subscription.
- **2026-04-26** — `feat(settings): Ollama toggle and host text input`
  (7c221d2). Settings panel gained the opt-in toggle plus text input
  for the ollama host URL.
- **2026-04-26** — `feat(config): bump to v2 with show_ollama and
  ollama_host` (a09a88a). `CONFIG_VERSION = 2`.
- **2026-04-26** — `feat(panel): adaptive orientation for vertical
  panels` (99d8c6e). `view()` in `app.rs:405-415` branches on
  `self.core.applet.is_horizontal()`.
- **2026-04-26** — `feat(panel): RAM, network, and disk sparklines
  (opt-in)` (f6a1a7d), `feat(sampler): disk sampler emitting
  read/write bytes/sec` (55953ba), `feat(sampler): network sampler
  emitting bytes/sec deltas` (d17dcf9), `feat(sparkline): Scale::AutoMax
  for unbounded metrics` (3789767). Added the unbounded-scale path for
  network and disk metrics.
- **2026-04-26** — `feat(settings): in-popup settings UI with live
  persistence` (1162433) and `feat(settings): expose warn/crit
  thresholds as sliders` (7abb01b). Established the
  Settings-inside-popup pattern with `cosmic-config` persistence.
- **2026-04-26** — `feat(gpu): probe_index for explicit GPU selection`
  (36ced0c), `feat(gpu): enumerate() returns descriptors for all
  detected GPUs` (d00cd3c), `feat(gpu): expose pdev() and is_nvidia()
  on each backend` (91bbcdb). Established multi-GPU selection in
  Settings.
- **2026-04-26** — `feat(gpu/procs): GpuProcessBackend trait + probe
  stubs` (1e0d01d), `feat(gpu/procs): NVML per-process
  implementation` (57babab), `feat(gpu/procs): DRM fdinfo per-process
  backend` (166367c), `feat(sampler): aggregator emits top CPU/GPU
  process lists` (8ebd4fd), `feat(sampler): top-N CPU process sampler
  via sysinfo` (a846646), `feat(popup): per-process drilldown for CPU
  and GPU` (faaa88a). Established the two-trait separation for GPU
  vs. GPU-process backends.
- **2026-04-26** — `feat(config): SystrkrConfig schema with
  cosmic-config derive` (36aee77) and `feat(app): subscribe to
  cosmic-config and apply changes live` (256e836). Established the
  derive + subscription pattern that all later config fields plug into.
- **2026-04-26** — `refactor(history): runtime-sized RingBuf with
  resize` (be9f7ef). Replaced the original fixed-size buffer with the
  current resizable form.

## Out of scope

- File-level code style, naming, formatting — those belong in
  `rustfmt`/`clippy` config and the shared coding standards.
- Per-file API documentation — belongs in `///` doc-comments.
- Release-by-release behavioral changes — belong in
  `docs/release-ledger.md` and `docs/releases/`.
- Open feature work and specs — belong in `~/homelab2-docs/` per the
  project conventions.

## How to use this doc in robot: runs

When running `robot:architectural-analysis` or `robot:code-review` on
this repo, this file is fed to the analysis as context. Each finding
gets classified as `Contradicts` an Intentional decision, `Restates`
a Known-deferred issue, or `New`. Only `New` and justified
`Contradicts` findings rise into the actionable section of the report.

The hash header above anchors this doc to a specific commit. If you
change the architecture, update both this doc AND the matching line in
CLAUDE.md in the same commit.
