default:
    just --list

build:
    cargo build --workspace

lint:
    cargo clippy --workspace -- -D warnings

check:
    cargo check --workspace

clean:
    cargo clean

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check
