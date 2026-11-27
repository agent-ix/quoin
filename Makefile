# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (C) 2026 Agent-IX

# Native Quoin build, test, and delivery entrypoints. The published executable
# is built from the Rust workspace; pnpm supplies semantic schema inputs.
# The published executable needs no source checkout or Node CLI shell.

RUST_DIR := $(CURDIR)/rust
CARGO_TARGET := $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),$(RUST_DIR)/target)
CARGO_TARGET_FLAG := --target-dir $(CARGO_TARGET)
QUIRE ?= quire
# Pass explicit roots for the complete isolated module set. In particular,
# spec-artifacts-process must be the current revision that registers
# IntendedUse and FeatureRiskRating. Example:
# make spec-validation QUIRE_MODULE_ARGS='--module /tmp/modules/spec-artifacts-iso --module /tmp/modules/spec-artifacts-process ...'
QUIRE_MODULE_ARGS ?=

.PHONY: build test lint format clean rust-build rust-fmt rust-lint rust-deny rust-test rust-gate build-inputs help evidence real-tool-test test-real mutation-test wave2-validation spec-validation test-sacm-interop

build: rust-build

# Workflow definitions ship beside plugin skills. This install supplies only
# semantic schema build inputs; the archived workflow provider is not used.
build-inputs:
	@pnpm install --frozen-lockfile --config.userconfig=/dev/null --registry=https://registry.npmjs.org/ --@agent-ix:registry=https://registry.npmjs.org/ --store-dir $(CARGO_TARGET)/pnpm-store || { rc=$$?; \
	  echo "build-inputs: 'pnpm install --frozen-lockfile' failed (exit $$rc); the pnpm error is above. It uses npmjs with a target-local store and an empty user config. Retry waits are bounded by pnpm-workspace.yaml." >&2; \
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

# Validate the complete specification bundle and require feature-risk coverage
# for every functional requirement.
spec-validation:
	@test -n "$(QUIRE_MODULE_ARGS)" || { echo "set QUIRE_MODULE_ARGS to explicit isolated module roots, including current spec-artifacts-iso and spec-artifacts-process" >&2; exit 2; }
	$(QUIRE) validate --scope . --okf --severity trace:feature-without-risk-rating=error $(QUIRE_MODULE_ARGS)

# Run the Rust suite under cargo-nextest (JUnit writer), add each test's trace
# tags to the report, and bind the run into spec/evidence with
# `quoin evidence record`. Needs `cargo install cargo-nextest --locked`.
# The tests' own exit status is returned after the run is recorded, so a red
# suite is recorded as red rather than skipped.
EVIDENCE_DIR := $(CARGO_TARGET)/nextest/evidence

evidence: build-inputs
	@command -v cargo-nextest >/dev/null || { echo "evidence: cargo-nextest is not installed (cargo install cargo-nextest --locked)"; exit 1; }
	@( cd $(RUST_DIR) && cargo build --locked --quiet $(CARGO_TARGET_FLAG) -p quoin-cli --bin quoin ) || exit 1; \
	rm -f $(EVIDENCE_DIR)/junit.xml $(EVIDENCE_DIR)/junit-traced.xml; \
	status=0; \
	( cd $(RUST_DIR) && cargo nextest run --workspace --locked --profile evidence $(CARGO_TARGET_FLAG) ) || status=$$?; \
	test -f $(EVIDENCE_DIR)/junit.xml || { echo "evidence: nextest wrote no JUnit report"; exit 1; }; \
	( cd $(RUST_DIR) && cargo run --locked --quiet $(CARGO_TARGET_FLAG) -p quoin-suite-trace -- \
		--repo $(CURDIR) --junit $(EVIDENCE_DIR)/junit.xml --out $(EVIDENCE_DIR)/junit-traced.xml ) || exit 1; \
	$(CARGO_TARGET)/debug/quoin evidence record --repo $(CURDIR) --suite SUITE-001 --kind Unit --adapter junit \
		--tool "cargo-nextest $$(cargo nextest --version | sed -n '1s/^cargo-nextest //;1s/ .*//p')" \
		--results $(EVIDENCE_DIR)/junit-traced.xml || exit 1; \
	exit $$status

# Focused Wave 2 verification: replay the ignored real-tool checks and the
# behavioral mutation kernels, then transcribe each result into Quoin's own
# evidence store. Run through locked-build.sh; the script retains raw outputs
# under target/wave2-evidence-mutation/.
# `make real-tool-test` is also supported on a clean checkout: build the
# recorder CLI before the runner invokes `quoin evidence record`.
real-tool-test: build-inputs
	cd $(RUST_DIR) && cargo build --locked -p quoin-cli --bin quoin $(CARGO_TARGET_FLAG)
	python3 tests/scripts/real_tool_test.py

# Narrow external parser check for the GSN Mermaid output. Run through
# locked-build.sh and supply the real parser explicitly; an unset path is a
# failure, never a skipped acceptance check.
test-real:
	@test -n "$(QUOIN_TEST_MMDR)" || { echo "test-real: QUOIN_TEST_MMDR=<path> is required"; exit 1; }
	cd $(RUST_DIR) && QUOIN_TEST_MMDR="$(QUOIN_TEST_MMDR)" cargo test --locked -p quoin-assurance --test gsn_mmdr -- --ignored

.PHONY: test-sacm-metamodel
test-sacm-metamodel:
	QUOIN_CARGO_TARGET="$(CARGO_TARGET)" tests/scripts/test_sacm_metamodel.sh

# Interop check against user-installed Assurance Forge 0.3.0-alpha.3 (MIT).
# The generated XMI is copied byte-for-byte to a .sacm.xml name for the tool's
# offline importer; no tool binary, plugin, or metamodel is stored in Quoin.
test-sacm-interop:
	@test -n "$(QUOIN_SACM_TOOL)" || { echo "QUOIN_SACM_TOOL must name the installed Assurance Forge MCP executable" >&2; exit 1; }
	@set -e; export_file="$(CARGO_TARGET)/sacm-interop/full-case.xmi"; \
	  import_file="$(CARGO_TARGET)/sacm-interop/full-case.sacm.xml"; \
	  rm -f "$$export_file" "$$import_file"; \
	  cd "$(RUST_DIR)" && QUOIN_SACM_INTEROP_FIXTURE="$$export_file" cargo test --locked $(CARGO_TARGET_FLAG) -p quoin-assurance --test tc_sacm_export export_external_interop_fixture -- --ignored --exact; \
	  cp "$$export_file" "$$import_file"; \
	  cd "$(CURDIR)" && QUOIN_SACM_TOOL="$(QUOIN_SACM_TOOL)" QUOIN_SACM_INTEROP_FIXTURE="$$import_file" python3 tests/scripts/test_sacm_interop.py

mutation-test:
	python3 tests/scripts/behavioral_mutation_test.py

wave2-validation: build-inputs
	cd $(RUST_DIR) && cargo build --locked --package quoin-cli --bin quoin $(CARGO_TARGET_FLAG)
	@set +e; $(MAKE) real-tool-test; real_status=$$?; $(MAKE) mutation-test; mutation_status=$$?; \
	  set -e; test $$real_status -eq 0 && test $$mutation_status -eq 0

# Set IX_FLOW_PACKAGE to a built local ix-flow package when testing a host API
# change before its npmjs release; dependencies still resolve from npmjs.
.PHONY: workflow-test workflow-test-runtime
workflow-test-runtime:
	@if [ -n "$(IX_FLOW_PACKAGE)" ]; then \
	  npm install --prefix $(CARGO_TARGET)/workflow-runner --userconfig /dev/null --cache=$(CARGO_TARGET)/npm-cache --registry=https://registry.npmjs.org/ --@agent-ix:registry=https://registry.npmjs.org/ --ignore-scripts --no-audit --no-fund "$(IX_FLOW_PACKAGE)"; \
	else \
	npm install --prefix $(CARGO_TARGET)/workflow-runner --userconfig /dev/null --cache=$(CARGO_TARGET)/npm-cache --registry=https://registry.npmjs.org/ --@agent-ix:registry=https://registry.npmjs.org/ --ignore-scripts --no-audit --no-fund @agent-ix/ix-flow@0.3.4; \
	fi

workflow-test: rust-build workflow-test-runtime
	cd $(RUST_DIR) && cargo run --locked $(CARGO_TARGET_FLAG) -p quoin-workflow-check -- smoke $(CARGO_TARGET)/debug/quoin $(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js $(CURDIR)
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::tc_fr020_relocated_plugin_launch_resolves_no_workflow -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::tc_fr020_missing_ix_flow_reports_supported_install_without_a_run -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::tc_fr020_relative_documents_resolve_under_subject_repo_with_spaces -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::fr095_review_workflow_enforces_live_contracts -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::formalize_failures_return_to_drafting_with_diagnostics -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::formalize_refuted_analysis_returns_to_drafting_with_retry_record -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::formalize_criterion_tests::a_scripted_valid_draft_reaches_awaiting_approval -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::formalize_criterion_tests::two_failures_then_a_valid_draft_reset_the_next_failure_count -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::design_gate_context_is_seeded_and_protected -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::design_gate_refuses_runner_without_host_api_before_run_creation -- --exact --test-threads=4
	cd $(RUST_DIR) && QUOIN_BINARY=$(CARGO_TARGET)/debug/quoin IX_FLOW_RUNNER=$(CARGO_TARGET)/workflow-runner/node_modules/@agent-ix/ix-flow/bin/ix-flow.js QUOIN_PLUGIN_ROOT=$(CURDIR) cargo test --locked $(CARGO_TARGET_FLAG) -j 4 -p quoin-workflow-check --features workflow-test smoke::tests::design_gate_refuses_incompatible_host_api_before_run_creation -- --exact --test-threads=4

# Real engineering-assurance, git and ssh-keygen against throwaway repositories
# and keys (PLAT-1789). Fails, never skips, when no verifier is named.
.PHONY: e2e-signing
e2e-signing:
	@test -n "$$QUOIN_E2E_EA" || { echo "e2e-signing: set QUOIN_E2E_EA to an engineering-assurance executable" >&2; exit 1; }
	cd $(RUST_DIR) && cargo test --locked $(CARGO_TARGET_FLAG) -p quoin-cli --test tc_plat_1789_signed_approval_chain -- --ignored

.PHONY: skill-test
skill-test:
	python3 tests/scripts/check_gap_analysis.py

.PHONY: skill-behavior-test
skill-behavior-test:
	python3 tests/scripts/check_gap_instruction_behavior.py
