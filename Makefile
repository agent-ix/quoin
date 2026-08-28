# =============================================================================
# quoin Makefile
# =============================================================================
# This Makefile provides backwards compatibility by delegating to pnpm scripts.
# All primary functionality is defined in package.json scripts section.
# Run 'pnpm run' to see all available scripts with descriptions.
# =============================================================================

# =============================================================================
# Core Development
# =============================================================================

.PHONY: build
build:
	pnpm run build

# `test` depends on `build`: oclif resolves commands from `./dist/commands`
# (package.json `oclif.commands`), so the dispatch-parity test sees an empty
# command list without a build. CI runs `make install` then `make test` with no
# build step in between, which is why that test passed locally — where a stale
# `dist/` lingers — and failed on every clean runner.
# `test` also depends on `validate` (quoin#183): spec docs are part of the gate.
# `test` also depends on `check-version` (quoin#196): the two version surfaces
# disagreed for every locally built binary, and only a BUILT artifact can show
# it — the source has no baked version to disagree with package.json.
.PHONY: test
test: build validate check-version
	QUIRE="$(abspath $(QUIRE))" pnpm run test

# Every surface that reports a version reports the same one, and a clean tag
# reports itself (quoin#196). The class of defect this catches shipped once
# already one repo over: quire-cli#52, where five consecutive tags shipped
# binaries all reporting the version before them. Run it before tagging.
# Re-pin the tier-2 answer key (quoin#200). NOT a re-measurement: the findings
# are claims about a specific tree, and carrying them to a different commit
# without reading it again asserts something nobody checked. This target only
# moves the pin — the adjudication is human work, and the diff belongs in the
# pull request. Same discipline as quire-rs's coverage_baseline.
# quoin dogfoods its own evidence store (quoin#206). It shipped one (FR-030)
# and did not use it on itself, while quire-rs dogfoods aggressively — and a
# store nobody runs against its own repository is a store whose failure modes
# nobody meets.
#
# `--ratchet` against the committed baseline: the 341 undischarged obligations
# are the accepted floor, so this can only improve. Not `--strict` in the
# default gate, because the floor is the whole backlog and failing on it would
# make the target useless on day one.
EVIDENCE_MODULE ?= $(HOME)/dev/spec-artifacts-process/spec_artifacts_process
# Pass 3 is a command, not a day (quoin#203). Runs the tool suite against the
# PINNED tier-2 corpus, scores it against the adjudicated answer key, and diffs
# against the checked-in baseline. A finding LOST is the failure; a finding
# gained is news.
#
# It does not replace the human pass. Every conclusion-changing finding of pass
# 2 came from somebody reading code, and a runner claiming otherwise would be
# the overclaim this programme exists to end. It replaces the RE-RUN.
BENCH_CORPUS ?= ../filament-ide-rs
.PHONY: battletest
battletest:
	@test -n "$(QUIRE)" || { echo "battletest requires QUIRE=/absolute/path/to/quire"; exit 2; }
	node scripts/battletest.mjs --quire $(QUIRE) --corpus $(BENCH_CORPUS) --module $(EVIDENCE_MODULE)

.PHONY: battletest-update
battletest-update:
	@test -n "$(QUIRE)" || { echo "battletest-update requires QUIRE=/absolute/path/to/quire"; exit 2; }
	node scripts/battletest.mjs --update --quire $(QUIRE) --corpus $(BENCH_CORPUS) --module $(EVIDENCE_MODULE)

# TIER 1: the seeded corpora, built, scanned and scored on every run. Cheap
# enough to gate on, unlike `battletest`, which needs a pinned external tree.
#
# The ratchet is one-way and quire-rs `scripts/bench.py`'s: a regression keeps
# the OLD baseline, so a bad run can never lower the bar, and `--update` is the
# only way a number moves. `QUIRE` overrides the binary — point it at a local
# build to score an engine that is not the installed one, which is the whole
# reason three SpecReviews once cited figures from a binary nobody checked.
# NOT a PATH lookup (agent-ix/quire-rs#265). The installed `quire` is
# whatever somebody put there — 0.29.0 here, predating `binding_census` —
# and scoring with it reported recall 0 on every coverage family, which
# reads as a corpus regression rather than a stale binary.
#
# CARGO_TARGET_DIR MOVES THE ARTIFACT AND THIS DEFAULT DID NOT FOLLOW IT. With
# it set, `cargo build` in quire-cli writes to `$CARGO_TARGET_DIR/debug/quire`
# and `../quire-cli/target/` keeps whatever was there before the variable was
# set. Measured on this machine: the in-repo path held an Aug-20 binary
# reporting `quire 0.23.0` while the real build reported
# `quire 0.30.2 (engine 00644b7)` — four days and one engine apart, and the
# default pointed at the stale one. Caught by quire-cli#68's provenance guard
# refusing a binary that cannot name its engine, which is the case for
# refusing rather than warning.
# Interactive convenience only. Governed evidence never consumes this default:
# `verification-stack.mjs` builds, snapshots, hashes, and passes its own binary.
QUIRE ?= $(shell command -v quire 2>/dev/null)
#
# `MODULES` overrides the DECLARATION the cases bind, which is the second axis
# (agent-ix/quoin#240). Empty means the corpus's own vendored `modules/`, so an
# ordinary run is unchanged. Two of Wave 3's six fixes are declaration-side and
# an engine-only before/after scores them `held` by construction — the same word
# it prints for a family that genuinely did not move.
#
#   make bench-tier1 QUIRE=<binary> MODULES=/path/to/pre-fix/modules
#
MODULES ?=
.PHONY: bench-tier1
bench-tier1:
	node scripts/verification-stack.mjs

.PHONY: bench-tier1-update
bench-tier1-update:
	node scripts/verification-stack.mjs --update

.PHONY: bench-tier1-experimental
bench-tier1-experimental:
	@test -n "$(QUIRE)" || { echo "usage: make bench-tier1-experimental QUIRE=/absolute/path/to/quire"; exit 2; }
	node scripts/bench-tier1.mjs --experimental --quire $(QUIRE) $(if $(MODULES),--modules $(MODULES))

.PHONY: verification-preflight
verification-preflight:
	node scripts/verification-stack.mjs --preflight

# THE LOCAL GREEN BAR. `bench-tier1` was invoked by nothing (agent-ix/quoin#244)
# -- not `test`, not any vitest case against the committed baseline, and CI is
# off by design during active development, so CI is not the answer. The ratchet
# ran when a human typed it, which between Wave 3 and Wave 4 was twice.
#
# `bench-tier1` LAST, because it is the slow leg: a lint or unit failure should
# come back in seconds rather than behind a two-minute corpus sweep.
#
# It names the engine it measured with. `quire --version` reports the CLI crate
# version, not the engine, and the installed 0.29.0 predates `binding_census` --
# it would score recall 0 on every coverage family and look like a collapse.
.PHONY: gate
gate: lint test
	@echo "gate: bench-tier1 against $(QUIRE)"
	@$(MAKE) bench-tier1

.PHONY: evidence-audit
evidence-audit:
	node bin/quoin.js evidence audit --repo . --module $(EVIDENCE_MODULE) --ratchet

# Re-transcribe this repository's own suite run into the store.
.PHONY: evidence-record
evidence-record:
	mkdir -p reports
	pnpm exec vitest run --reporter=junit --outputFile=reports/junit.xml
	node bin/quoin.js evidence record --suite SUITE-001 \
	  --commit "$$(git rev-parse HEAD)" --tool vitest --adapter junit \
	  --results reports/junit.xml --kind Unit --repo . --module $(EVIDENCE_MODULE)

.PHONY: answer-key-repin
answer-key-repin:
	@test -n "$(SHA)" || { echo "usage: make answer-key-repin SHA=<commit>"; exit 2; }
	@echo "Re-pinning the answer key to $(SHA)."
	@echo "STOP: re-read the corpus at that commit and re-adjudicate every finding."
	@echo "Moving the pin without re-adjudicating turns the recall denominator into fiction."
	python3 -c "import json,sys; p='bench/answer-key.json'; d=json.load(open(p)); d['pinned_sha']='$(SHA)'; json.dump(d, open(p,'w'), indent=2); open(p,'a').write('\n')"

.PHONY: check-version
check-version: build
	node scripts/check-version-agreement.mjs

# Spec validation gate, mirroring quire-rs's `make validate` (quoin#183):
# every spec/, plan/ and reviews/ document must pass `quire validate`
# structurally, so an unadmitted matrix cell fails here instead of surfacing
# in an unrelated session. Runs the repo-pinned @agent-ix/quire-cli
# devDependency (not a host binary), so clean CI runners gate too.
# Grammar warnings stay advisory (no --strict); structural failures exit 1.
#
# The modules come first, and from THIS build. `quire validate` falls back to
# `quoin module ensure-defaults` when `~/.ix` holds no modules, and reaches for
# a `quoin` on PATH — which a clean runner does not have, so every CI run since
# 2026-08-21 died with "quoin not found on PATH" while a developer machine with
# a populated `~/.ix` passed. Installing them with the CLI the repo just built
# also means a module set is never validated against a stale host binary.
.PHONY: validate
validate: build
	@test -n "$(QUIRE)" || { echo "validate requires QUIRE=/absolute/path/to/quire"; exit 2; }
	node bin/quoin.js module ensure-defaults
	$(QUIRE) validate "spec/**/*.md" "plan/**/*.md" "reviews/*.md"

.PHONY: test-json
test-json:
	pnpm run test:json

# Agent-pty evals (drive the REAL claude agent; cost tokens + minutes — opt-in).
# MODEL pins the agent model so token counts compare; REPEATS aggregates noise.
MODEL ?= sonnet
REPEATS ?= 1

.PHONY: evals
evals:
	pnpm --dir ../cli-agent-evals run build
	node ../cli-agent-evals/bin/cli-evals.js run --suite ./cli-agent-evals.config.mjs --canary --agent claude --model $(MODEL) --repeats $(REPEATS)

.PHONY: evals-all
evals-all:
	pnpm --dir ../cli-agent-evals run build
	node ../cli-agent-evals/bin/cli-evals.js run --suite ./cli-agent-evals.config.mjs --all --agent claude --model $(MODEL) --repeats $(REPEATS)

# Community install smoke test (clean-room Docker: public npm + agent plugins).
# Verifies an outside developer can `npm i -g @agent-ix/quoin` and install the
# plugin into Claude Code, OpenAI Codex, opencode, and GitHub Copilot.
# See smoke/README.md.
.PHONY: install-smoke
install-smoke:
	./smoke/run.sh

.PHONY: lint
lint:
	pnpm run lint

.PHONY: audit-tool-drift
audit-tool-drift:
	pnpm run audit:tool-drift
	pnpm run test:tool-drift
	pnpm run test:verification-stack

.PHONY: format
format:
	pnpm run format

.PHONY: format-check
format-check:
	pnpm run format:check

.PHONY: clean
clean:
	pnpm run clean

# =============================================================================
# Package Management
# =============================================================================

.PHONY: install
install:
	pnpm install

.PHONY: update-lock
update-lock:
	pnpm run update-lock

.PHONY: add-packages
add-packages:
	@echo "Adding packages: $(PACKAGES)"
	pnpm run pkg:add $(PACKAGES)

.PHONY: add-dev-packages
add-dev-packages:
	@echo "Adding dev packages: $(PACKAGES)"
	pnpm run pkg:add-dev $(PACKAGES)

.PHONY: update-packages
update-packages:
	pnpm run pkg:update

.PHONY: experimental-update-packages-latest
experimental-update-packages-latest:
	@echo "NONCANONICAL: this deliberately resolves moving package versions and cannot produce governed evidence."
	pnpm run experimental:pkg:update-latest

.PHONY: use-local
use-local:
	@echo "Switching $(p) to local..."
	pnpm run pkg:use-local $(p)

.PHONY: use-upstream
use-upstream:
	@echo "Switching $(p) to upstream..."
	pnpm run pkg:use-upstream $(p)

.PHONY: refresh-local
refresh-local:
	pnpm run pkg:refresh-local

# =============================================================================
# Versioning & Info
# =============================================================================

.PHONY: version
version:
	@pnpm run version

.PHONY: info
info:
	@pnpm run info

# =============================================================================
# Docker & Publishing
# =============================================================================

.PHONY: docker-build
docker-build:
	pnpm run docker:build


.PHONY: tags
tags:
	@pnpm run tags

# =============================================================================
# Test Results CLI
# =============================================================================
# Usage: make test-results-summary REPORT=report.json
#        make test-results-groups REPORT=report.json
#        make test-results-detail REPORT=report.json TEST="test name"
#        make test-results-find REPORT=report.json PATTERN="test_"
#        make test-results-failed REPORT=report.json
#        make test-results-errors REPORT=report.json
#        make test-results-warnings REPORT=report.json

REPORT ?= report.json

.PHONY: test-results-summary
test-results-summary:
	pnpm run test-results:summary $(REPORT)

.PHONY: test-results-groups
test-results-groups:
	pnpm run test-results:groups $(REPORT)

.PHONY: test-results-detail
test-results-detail:
	pnpm run test-results:detail $(REPORT) "$(TEST)"

.PHONY: test-results-find
test-results-find:
	pnpm run test-results:find $(REPORT) "$(PATTERN)"

.PHONY: test-results-failed
test-results-failed:
	pnpm run test-results:failed $(REPORT)

.PHONY: test-results-errors
test-results-errors:
	pnpm run test-results:errors $(REPORT)

.PHONY: test-results-warnings
test-results-warnings:
	pnpm run test-results:warnings $(REPORT)

# =============================================================================
# Help
# =============================================================================

.PHONY: help
help:
	@echo "quoin Makefile - Backwards compatibility wrapper"
	@echo ""
	@echo "This Makefile delegates to pnpm scripts defined in package.json"
	@echo "Run 'pnpm run' to see all available scripts"
	@echo ""
	@echo "Common targets:"
	@echo "  make build              - Build TypeScript"
	@echo "  make test               - Run tests"
	@echo "  make lint               - Run linter"
	@echo "  make format             - Format code"
	@echo "  make clean              - Remove build artifacts"
	@echo "  make install            - Install dependencies"
	@echo "  make version            - Show computed version"
	@echo "  make info               - Show git info"
