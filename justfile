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

PREFIX := env_var_or_default("PREFIX", env_var("HOME") + "/.local")

install: build
    install -Dm755 target/release/galaxy-systrkr "{{PREFIX}}/bin/galaxy-systrkr"
    install -Dm644 data/com.goldengod503.GalaxySysTrkr.desktop "{{PREFIX}}/share/applications/com.goldengod503.GalaxySysTrkr.desktop"
    install -Dm644 data/icons/com.goldengod503.GalaxySysTrkr.svg "{{PREFIX}}/share/icons/hicolor/scalable/apps/com.goldengod503.GalaxySysTrkr.svg"
    @echo "Installed to {{PREFIX}}. Add the applet via cosmic-settings → Panel."

uninstall:
    rm -f "{{PREFIX}}/bin/galaxy-systrkr"
    rm -f "{{PREFIX}}/share/applications/com.goldengod503.GalaxySysTrkr.desktop"
    rm -f "{{PREFIX}}/share/icons/hicolor/scalable/apps/com.goldengod503.GalaxySysTrkr.svg"
