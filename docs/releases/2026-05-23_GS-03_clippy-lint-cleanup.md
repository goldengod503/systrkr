# GS-03 — Clippy lint cleanup (Rust 1.93 / clippy strictness bump)

**Date:** 2026-05-23
**Branch:** `main`
**Commit:** `e3b1de3` — `style: clear new clippy lints (collapsible_if, useless_conversion, etc.)`

## Summary

Cleared the 13 clippy errors that turned the `just clippy` gate red on the
current toolchain. All findings were lint-strictness regressions, not
behavioral defects — Rust 1.93 / clippy added stricter `collapsible_if`
suggestions that prefer Rust 2024's `&& let` chain syntax, plus a handful
of long-standing `useless_conversion` / `needless_return` / `manual_find`
/ `question_mark` / `duplicated_attributes` instances that earlier
clippy releases did not flag. Closes the "Clippy was red at baseline
with 16 pre-existing errors; not addressed in this stream" open item
called out in GS-02's Test Plan.

## Scope

**Files Modified**
- `src/app.rs` — collapsed `if let (Some, Some) = .. { if total > 0 { .. } }`
  into a `&& let` chain (`app.rs:121-126`); dropped tail `return` in the
  `Message::TogglePopup` arm so the arm yields the `Task` as its block
  expression (`app.rs:150-172`); rewrote `detect_system_monitor`'s
  manual `for`-loop probe as `candidates.into_iter().find(|bin| which(bin))`
  (`app.rs:469-472`).
- `src/popup.rs` — dropped three `.into()` calls on values already of
  type `Element<'_, Message>` (`popup.rs:23`, `:26`, `:44`).
- `src/sampler/gpu/amd.rs` — collapsed nested `if let`s in
  `read_amdgpu_temp` into a single `&& let` chain (`amd.rs:110-114`).
- `src/sampler/gpu/intel.rs` — replaced `if find_render_engine(card).is_none() { return None; }`
  with `find_render_engine(card)?;` in `probe_specific` (`intel.rs:39`).
- `src/sampler/gpu/mod.rs` — collapsed `if let Ok(lib) = .. { if let Ok(count) = .. { .. } }`
  in `enumerate` (NVIDIA path) into a `&& let` chain (`mod.rs:78-96`).
- `src/sampler/gpu/nvml.rs` — dropped the redundant file-level
  `#![cfg(feature = "nvidia")]`; the parent `mod.rs` already
  `#[cfg]`-gates the `mod nvml;` line.
- `src/sampler/gpu/procs/mod.rs` — collapsed the three-deep
  `if is_nvidia { if let Some(idx) = .. { if let Some(b) = .. } }` in
  `probe` into a single `&& let` chain (`procs/mod.rs:34-40`).
- `src/sampler/gpu/procs/nvml.rs` — same redundant file-level
  `#![cfg(feature = "nvidia")]` removal.

**Files Created**
- `docs/releases/2026-05-23_GS-03_clippy-lint-cleanup.md` — this file.

## Behavioral Impact

**No behavior change.** Every fix is either:

- A pure syntactic restructuring that preserves identical control flow
  (`collapsible_if` rewrites — `&& let` chains short-circuit the same
  way as nested `if let`s; `question_mark` rewrite — `?` on `Option`
  returns `None` identically to `if .is_none() { return None }`;
  `manual_find` rewrite — `.into_iter().find()` returns the first
  matching `&'static str` in the same order as the loop).
- A no-op type tidy (`useless_conversion` — removing `.into()` calls
  whose source and target types were already equal).
- A dead-line removal (`duplicated_attributes` — file-level
  `#![cfg(feature = "nvidia")]` was redundant; the parent
  `mod nvml;` line is already cfg-gated, so the module never compiles
  without the feature regardless).
- A whitespace tidy (`needless_return` — last expression of a match
  arm, semicolon dropped).

## Test Plan

- Build: `cargo check --all-features` clean.
- Tests: `cargo test --all-features` — **39 passed, 0 failed**.
- Clippy (gate): `just clippy` (= `cargo clippy --all-features -- -D warnings`)
  clean — no warnings, no errors.
- Clippy (AMD-only path): `cargo clippy --no-default-features -- -D warnings`
  clean — confirms the `duplicated_attributes` removals did not silently
  drop the `nvidia` gate (the `nvml` modules still don't compile without
  the feature, courtesy of the parent `#[cfg]`).
- Manual: not run — pure lint cleanup, no UX shape change.

## Docs Updated

- `docs/release-ledger.md` — GS-03 row added.

No `docs/ARCHITECTURE.md` SHA bump: no structural drift introduced by
this commit (no files added/removed, no module boundaries shifted, no
public symbols changed). The verified SHA stays at `456bee7`.

## Rollback Plan

```bash
git revert e3b1de3
```

Single commit, no schema changes, no behavioral changes — clean revert.

## Open Questions / Decisions

None. The new `&& let` chain syntax is already in use elsewhere in the
codebase (e.g., `src/sampler/cpu.rs`), so this commit aligns the
remaining stragglers with the prevailing style.
