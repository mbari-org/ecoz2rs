# A convenient default for development: test and format
default: test format

# default + clippy; good to run before committing changes
all: default clippy

# List recipes
list:
	@just --list --unsorted

# Run tests
test:
	cargo test

# Run espclient (e.g.:  just run --help)
run *args='':
	cargo run -- {{ args }}

# Format source code
format:
	cargo fmt

# Run clippy
clippy:
	cargo clippy --no-deps

# Build release
release:
	cargo build --release

# Install locally
install: release
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
