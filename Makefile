# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

# Native Quoin build, test, and delivery entrypoints. The published executable
# is built from the Rust workspace; pnpm supplies semantic schema inputs.
# The published executable needs no source checkout or Node CLI shell.

RUST_DIR := $(CURDIR)/rust
CARGO_TARGET := $(RUST_DIR)/target
CARGO_TARGET_FLAG := --target-dir $(CARGO_TARGET)

.PHONY: build test lint format clean rust-build rust-fmt rust-lint rust-deny rust-test rust-gate build-inputs help evidence

build: rust-build

# Workflow definitions ship beside plugin skills. This install supplies only
# semantic schema build inputs; the archived workflow provider is not used.
build-inputs:
	@pnpm install --frozen-lockfile || { rc=$$?; \
	  echo "build-inputs: 'pnpm install --frozen-lockfile' failed (exit $$rc); the pnpm error is above. If it is a connection error, this step needs the registries in your npm config ('pnpm config get registry'): even a lockfile satisfied from the store re-checks package metadata. Retry waits are bounded by pnpm-workspace.yaml." >&2; \
	  exit $$rc; }

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
	@echo "Native Quoin targets: build, test, lint, format, clean, evidence"

# Run the Rust suite under cargo-nextest (JUnit writer), add each test's trace
# tags to the report, and bind the run into spec/evidence with
# `quoin evidence record`. Needs `cargo install cargo-nextest --locked`.
# The tests' own exit status is returned after the run is recorded, so a red
# suite is recorded as red rather than skipped.
EVIDENCE_DIR := $(CARGO_TARGET)/nextest/evidence

evidence: build-inputs
	@command -v cargo-nextest >/dev/null || { echo "evidence: cargo-nextest is not installed (cargo install cargo-nextest --locked)"; exit 1; }
	@( cd $(RUST_DIR) && cargo build --locked --quiet $(CARGO_TARGET_FLAG) -p quoin-cli --bin quoin ) || exit 1; \
	status=0; \
	( cd $(RUST_DIR) && cargo nextest run --workspace --locked --profile evidence $(CARGO_TARGET_FLAG) ) || status=$$?; \
	test -f $(EVIDENCE_DIR)/junit.xml || { echo "evidence: nextest wrote no JUnit report"; exit 1; }; \
	( cd $(RUST_DIR) && cargo run --locked --quiet $(CARGO_TARGET_FLAG) -p quoin-suite-trace -- \
		--repo $(CURDIR) --junit $(EVIDENCE_DIR)/junit.xml --out $(EVIDENCE_DIR)/junit-traced.xml ) || exit 1; \
	$(CARGO_TARGET)/debug/quoin evidence record --repo $(CURDIR) --suite SUITE-001 --kind Unit --adapter junit \
		--tool "cargo-nextest $$(cargo nextest --version | sed -n '1s/^cargo-nextest //;1s/ .*//p')" \
		--results $(EVIDENCE_DIR)/junit-traced.xml || exit 1; \
	exit $$status

.PHONY: workflow-test workflow-test-runtime
workflow-test-runtime:
	npm install --prefix $(CARGO_TARGET)/workflow-runner --registry=https://registry.npmjs.org/ --@agent-ix:registry=https://registry.npmjs.org/ --ignore-scripts --no-audit --no-fund @agent-ix/ix-flow@0.3.1

workflow-test: rust-build workflow-test-runtime
	cd $(RUST_DIR) && cargo run --locked $(CARGO_TARGET_FLAG) -p quoin-workflow-check -- smoke $(CARGO_TARGET)/debug/quoin $(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js $(CURDIR)
