# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

# Native Quoin build, test, and delivery entrypoints. The published executable
# is built from the Rust workspace; no Node package manager or oclif runner is
# part of this path.

RUST_DIR := $(CURDIR)/rust
CARGO_TARGET := $(RUST_DIR)/target
CARGO_TARGET_FLAG := --target-dir $(CARGO_TARGET)

.PHONY: build test lint format clean rust-build rust-fmt rust-lint rust-deny rust-test rust-gate help

build: rust-build

test: rust-gate

lint: rust-lint

format: rust-fmt

clean:
	cd $(RUST_DIR) && cargo clean $(CARGO_TARGET_FLAG)

rust-build:
	cd $(RUST_DIR) && cargo build --workspace --locked $(CARGO_TARGET_FLAG)

rust-fmt:
	cd $(RUST_DIR) && cargo fmt --all

rust-lint:
	cd $(RUST_DIR) && cargo fmt --all --check
	cd $(RUST_DIR) && cargo clippy --workspace --all-targets --all-features --locked $(CARGO_TARGET_FLAG) -- -D warnings

rust-deny:
	cd $(RUST_DIR) && cargo deny check

rust-test:
	cd $(RUST_DIR) && cargo test --workspace --locked $(CARGO_TARGET_FLAG)

rust-gate: rust-lint rust-deny rust-test
	@echo "rust-gate: fmt, clippy -D warnings, cargo deny and tests passed"

help:
	@echo "Native Quoin targets: build, test, lint, format, clean"
