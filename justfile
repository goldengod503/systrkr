default:
    @just --list

build:
    cargo build --release

run:
    cargo run

check:
    cargo check --all-features

test:
    cargo test --all-features

fmt:
    cargo fmt

clippy:
    cargo clippy --all-features -- -D warnings
