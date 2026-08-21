# List recipes
list:
	@just --list --unsorted

# good to run before committing changes
all: test format clippy

# Run check
check:
	cargo check

## Benchmarking procedure
##
##   just bench                     # quick numbers; "change" = vs last run, unreliable
##   just bench-save <name>         # fix a reference point (once, on a commit worth anchoring)
##   just bench-cmp <name>          # compare against it, repeatably; never moves the reference
##   just bench-cmp-lenient <name>  # same, when the baseline predates a new bench
##   just bench-baselines           # list what's saved  (note: `just clean` wipes them)
##
## Read the change percentage, not criterion's verdict: at long measurement
## times it labels ~1% drift "regressed". Under ~3% is noise on this machine.
##
## Most reliable: variants in one group (lpca1/2/3, lpca_c) share machine
## conditions, so their ratios hold even when absolute numbers drift.

# Quick benchmark run: absolute numbers + HTML report
bench:
	cargo bench

# Save current results as a named baseline (long measurement time = low-noise reference)
bench-save name='main':
	cargo bench --bench my_benchmark -- --save-baseline {{name}} --measurement-time 15

# Compare current code against a named baseline (does NOT overwrite it)
bench-cmp name='main':
	cargo bench --bench my_benchmark -- --baseline {{name}} --measurement-time 15

# Like bench-cmp but tolerates benchmarks missing from the baseline
bench-cmp-lenient name='main':
	cargo bench --bench my_benchmark -- --baseline-lenient {{name}} --measurement-time 15

# List saved baselines
bench-baselines:
	@find target/criterion -maxdepth 3 -mindepth 3 -type d \
	  ! -name new ! -name base ! -name change ! -name report \
	  | sed 's|target/criterion/||' | sort

# Run tests
test:
	cargo test

# Run tests with --nocapture
test-nocapture *args='':
    cargo test -- --nocapture {{args}}

# Run espclient (e.g.:  just run --help)
run *args='':
	cargo run -- {{ args }}

# wrkflw validate
wrkflw_validate:
    wrkflw validate

# List git tags
tags:
  git tag -l | sort -V | tail

# Create and push git tag
tag-and-push:
  #!/usr/bin/env bash
  version=$(cat Cargo.toml | grep version | head -1 | cut -d\" -f2)
  echo "tagging and pushing v${version}"
  git tag v${version}
  git push origin v${version}

# Clean
clean:
  cargo clean

# Format source code
format:
	cargo fmt

# Run clippy
clippy:
	cargo clippy --no-deps -- -D warnings

# Build release
build:
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
