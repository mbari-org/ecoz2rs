# A convenient default for development: test and format
default: test format

# default + clippy; good to run before committing changes
all: default clippy

# List recipes
list:
	@just --list --unsorted

# Run check
check:
	cargo check

# Run benchmarks (then open target/criterion/report/index.html)
bench:
	cargo bench

# Run tests
test:
	cargo test

# Run tests with --nocapture
test-nocapture *args='':
    cargo test -- --nocapture {{args}}

# Run espclient (e.g.:  just run --help)
run *args='':
	cargo run -- {{ args }}

# Clean
clean:
  cargo clean

# Format source code
format:
	cargo fmt

# Run clippy
clippy:
	cargo clippy --no-deps

# Build release
release:
	cargo build --release

# Build release with RUSTFLAGS="-C target-cpu=native"
release-native:
	RUSTFLAGS="-C target-cpu=native" cargo build --release

# Install locally
install: release-native
	cargo install --path .

# (cargo install --locked cargo-outdated)
# Show outdated dependencies
outdated:
	cargo outdated --root-deps-only

# (cargo install --locked cargo-udeps)
# Find unused dependencies
udeps:
	cargo +nightly udeps

# cargo update
update:
	cargo update
