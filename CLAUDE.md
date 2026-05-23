# galaxy-systrkr — Project Notes

Project-specific context for Claude Code sessions in this repo. The
PC-global and homelab-shared CLAUDE.md files load on top of this one
(see `~/.claude/CLAUDE.md`).

**Last verified against commit:** 456bee7 (2026-05-23)

## What this is

A native [COSMIC](https://system76.com/cosmic) panel applet that shows
live CPU and GPU usage as sparklines plus a system-details popup.
Single Rust binary, libcosmic / Iced GUI.

## Architecture reference

See `docs/ARCHITECTURE.md` for module layout, intentional decisions,
and known-deferred issues. Robot analytical skills cross-check the
verified SHA above against the matching line in `docs/ARCHITECTURE.md`.

## Build / dev commands

```bash
just check       # cargo check --all-features
just test        # cargo test --all-features
just clippy      # cargo clippy --all-features -- -D warnings
just run         # run the applet standalone for sanity-checking
just install     # install to ~/.local
```

For AMD-only systems: `cargo build --release --no-default-features`.

## Where things live

- Specs / plans for this project: `~/homelab2-docs/specs/galaxy-systrkr/`,
  `~/homelab2-docs/plans/galaxy-systrkr/` (per shared homelab convention).
- Release docs: `docs/releases/` (in-repo).
- Release ledger: `docs/release-ledger.md`.
