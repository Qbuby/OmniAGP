.PHONY: all build lint test clean

all: build lint test

build:
	cargo build --workspace

lint:
	cargo clippy --workspace -- -D warnings

test:
	cargo test --workspace

clean:
	cargo clean
