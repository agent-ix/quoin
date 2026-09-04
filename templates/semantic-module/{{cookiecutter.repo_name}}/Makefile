# =============================================================================
# {{ cookiecutter.repo_name }} — Quire semantic module
# =============================================================================
# Tasks live in pyproject.toml under [tool.poe.tasks]; these targets delegate.
# Run `make help` for the list.
# =============================================================================

POETRY = poetry
POE = $(POETRY) run poe

.PHONY: help
help:
	@echo "Setup"
	@echo "  make bootstrap      - install the toolchain and emit schemas/ + manifest digests"
	@echo "  make install        - install Python and Node dependencies"
	@echo "  make dev-quire      - install the Quire wheel the semantic tests need"
	@echo ""
	@echo "Gate"
	@echo "  make gate           - validate, lint, schema drift check, tests (the green bar)"
	@echo "  make validate       - quire validate over spec/"
	@echo "  make lint           - ruff + black + schema drift check"
	@echo "  make test           - the verification suite"
	@echo ""
	@echo "Schemas"
	@echo "  make schemas        - emit {{ cookiecutter.package_name }}/schemas/ from typespec/main.tsp"
	@echo "  make schemas-check  - fail on schema, toolchain, or digest drift"
	@echo ""
	@echo "Build"
	@echo "  make build          - build the Python distribution"
	@echo "  make pack           - build the npm tarball"
	@echo "  make clean          - remove build artifacts"

# =============================================================================
# Setup
# =============================================================================

.PHONY: install
install:
	$(POETRY) install
	npm install

# One command from a fresh clone to a repository whose gate can pass. The emit
# step is here rather than in the cookiecutter hook because emitting needs
# `@agent-ix/semantic-core` from a registry, and a template that reached the
# network while rendering would render differently offline.
.PHONY: bootstrap
bootstrap: install schemas
	@echo ""
	@echo "Bootstrapped. `make dev-quire` next, then `make gate`."

# The semantic rows need the Quire wheel exposing `extract_semantic`. It is not
# a declared dependency: no index this repository may commit against serves it
# (agent-ix/quire-rs#392). The suite FAILS without it — it never skips, because
# a skipped row is not coverage.
.PHONY: dev-quire
dev-quire:
	$(POE) dev-quire

# =============================================================================
# Gate
# =============================================================================

# THE GREEN BAR. Every leg fails the whole target. `validate` first because a
# structural spec error is the cheapest failure to read; `test` last because it
# is the slowest.
.PHONY: gate
gate: validate lint test
	@echo "gate: spec validation, lint, schema drift check and the suite all passed"

.PHONY: validate
validate:
	quire validate --scope . "spec/**/*.md"

.PHONY: lint
lint:
	$(POE) lint

.PHONY: format
format:
	$(POE) format

.PHONY: test
test:
	$(POE) test

# =============================================================================
# Schemas
# =============================================================================

.PHONY: schemas
schemas:
	$(POE) schemas

.PHONY: schemas-check
schemas-check:
	$(POE) schemas-check

# =============================================================================
# Build
# =============================================================================

.PHONY: build
build:
	$(POE) build

.PHONY: pack
pack:
	npm pack

.PHONY: clean
clean:
	$(POE) clean

.PHONY: version
version:
	@$(POE) version

.PHONY: info
info:
	@$(POE) info
