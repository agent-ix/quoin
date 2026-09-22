# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

# Native Quoin build, test, and delivery entrypoints. The published executable
# is built from the Rust workspace; no Node package manager or oclif runner is
# part of this path.

RUST_DIR := $(CURDIR)/rust
CARGO_TARGET := $(RUST_DIR)/target
CARGO_TARGET_FLAG := --target-dir $(CARGO_TARGET)

.PHONY: build test lint format clean rust-build rust-fmt rust-lint rust-deny rust-test rust-gate workflow-assets help

build: rust-build

# `quoin review`, `quoin matrix` and `quoin to-plan` (rust/crates/quoin-cli/src/flow.rs)
# launch the workflows @agent-ix/ix-spec-workflows publishes; nothing here vendors a
# copy of that package (PLAT-157). `--frozen-lockfile` fails loud on a lockfile that
# does not match package.json rather than silently re-resolving.
#
# `rust/crates/quoin-semantic/build.rs` also depends on this: it embeds the
# semantic contract straight out of `node_modules/@agent-ix/*` (PLAT-887's
# de-vendoring), so `cargo build`/`cargo test` on quoin-semantic hard-fail
# without a populated `node_modules`. `rust-build`, `rust-lint` and `rust-test`
# all depend on this target so a plain `make build` on a clean checkout does
# not need an undocumented separate step.
workflow-assets:
	pnpm install --frozen-lockfile

test: rust-gate

lint: rust-lint

format: rust-fmt

clean:
	cd $(RUST_DIR) && cargo clean $(CARGO_TARGET_FLAG)

rust-build: workflow-assets
	cd $(RUST_DIR) && cargo build --workspace --locked $(CARGO_TARGET_FLAG)

rust-fmt:
	cd $(RUST_DIR) && cargo fmt --all

rust-lint: workflow-assets
	cd $(RUST_DIR) && cargo fmt --all --check
	cd $(RUST_DIR) && cargo clippy --workspace --all-targets --all-features --locked $(CARGO_TARGET_FLAG) -- -D warnings

rust-deny:
	cd $(RUST_DIR) && cargo deny check

rust-test: workflow-assets
	cd $(RUST_DIR) && cargo test --workspace --locked $(CARGO_TARGET_FLAG)

rust-gate: rust-lint rust-deny rust-test
	@echo "rust-gate: fmt, clippy -D warnings, cargo deny and tests passed"

help:
	@echo "Native Quoin targets: build, test, lint, format, clean"
