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
.PHONY: test
test: build validate
	pnpm run test

# Spec validation gate, mirroring quire-rs's `make validate` (quoin#183):
# every spec/, plan/ and reviews/ document must pass `quire validate`
# structurally, so an unadmitted matrix cell fails here instead of surfacing
# in an unrelated session. Runs the repo-pinned @agent-ix/quire-cli
# devDependency (not a host binary), so clean CI runners gate too.
# Grammar warnings stay advisory (no --strict); structural failures exit 1.
.PHONY: validate
validate:
	pnpm run validate

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

.PHONY: update-packages-latest
update-packages-latest:
	pnpm run pkg:update-latest

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
