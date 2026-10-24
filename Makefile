# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

# Native Quoin build, test, and delivery entrypoints. The published executable
# is built from the Rust workspace; pnpm supplies semantic schema inputs.
# The published executable needs no source checkout or Node CLI shell.

RUST_DIR := $(CURDIR)/rust
CARGO_TARGET := $(RUST_DIR)/target
CARGO_TARGET_FLAG := --target-dir $(CARGO_TARGET)

.PHONY: build test lint format clean rust-build rust-fmt rust-lint rust-deny rust-test rust-gate build-inputs help

build: rust-build

# Workflow definitions ship beside plugin skills. This install supplies only
# semantic schema build inputs; the archived workflow provider is not used.
build-inputs:
	pnpm install --frozen-lockfile

test: rust-gate

lint: rust-lint

format: rust-fmt

clean:
	cd $(RUST_DIR) && cargo clean $(CARGO_TARGET_FLAG)

rust-build: build-inputs
	cd $(RUST_DIR) && cargo build --workspace --locked $(CARGO_TARGET_FLAG)

rust-fmt:
	cd $(RUST_DIR) && cargo fmt --all

rust-lint: build-inputs
	cd $(RUST_DIR) && cargo fmt --all --check
	cd $(RUST_DIR) && cargo clippy --workspace --all-targets --all-features --locked $(CARGO_TARGET_FLAG) -- -D warnings

rust-deny:
	cd $(RUST_DIR) && cargo deny check

rust-test: build-inputs
	cd $(RUST_DIR) && cargo test --workspace --locked $(CARGO_TARGET_FLAG)

rust-gate: rust-lint rust-deny rust-test workflow-test
	@echo "rust-gate: fmt, clippy -D warnings, cargo deny, tests and installed workflow scenarios passed"

help:
	@echo "Native Quoin targets: build, test, lint, format, clean"

.PHONY: workflow-test workflow-test-runtime
workflow-test-runtime:
	npm install --prefix $(CARGO_TARGET)/workflow-runner --registry=https://registry.npmjs.org/ --@agent-ix:registry=https://registry.npmjs.org/ --ignore-scripts --no-audit --no-fund @agent-ix/ix-flow@0.3.1

workflow-test: rust-build workflow-test-runtime
	cd $(RUST_DIR) && cargo run --locked $(CARGO_TARGET_FLAG) -p quoin-workflow-check -- smoke $(CARGO_TARGET)/debug/quoin $(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js $(CURDIR)
