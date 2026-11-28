#!/usr/bin/env bash
set -euo pipefail

RUN_DIR=/home/peter/dev/.scratchpad/plat-1822-real-tool-runs
REPO=/home/peter/dev/worktrees/quoin-plat1822-release-exact
RUST_DIR="$REPO/rust"
RELEASE_SHA=315afbec3935a9a29dae29f2a596c74b6df0e738
RELEASE_SHORT=${RELEASE_SHA:0:7}
TARGET_DIR=/builds/PLAT-1822/target
LOCKED_BUILD=/home/peter/.local/bin/locked-build
QUOIN_BIN="$TARGET_DIR/debug/quoin"
EA_BIN=/home/peter/dev/engineering-assurance/.worktrees/plat-1822-ea-0111-hotfix/target/debug/engineering-assurance
QUIRE_BIN=/builds/PLAT-1822/tools/node_modules/.bin/quire
IX_FLOW_BIN=/builds/PLAT-1822/tools/node_modules/.bin
IX_FLOW_RUNNER=/builds/PLAT-1822/tools/node_modules/@agent-ix/ix-flow/bin/ix-flow.js
FILAMENT_ANALYZE_BIN=/home/peter/dev/filament-analyze/.worktrees/codex-modules-plat1822-main/target/debug/filament-analyze
SPEC_ARTIFACTS_PROCESS_DIR=/home/peter/dev/worktrees/plat1822-process-v0380-exact
SPEC_OBJECTS_SAFETY_DIR=/home/peter/dev/spec-objects-safety
SPEC_OBJECTS_SECURITY_DIR=/home/peter/dev/worktrees/quoin-release-modules-20261127/spec-objects-security/spec_objects_security
PDFTOTEXT=/home/peter/.local/bin/pdftotext
OPENVEX_SCHEMA_PATH="$RUN_DIR/openvex-0.2.0-schema.json"
CYCLONEDX_SCHEMA_PATH="$RUN_DIR/cyclonedx-1.7-bom-schema.json"
TMPDIR="${TMPDIR:-/builds/PLAT-1822/tmp}"
CLEAN_HOME="$RUN_DIR/clean-home-$RELEASE_SHORT"
CLEAN_IX_HOME="$CLEAN_HOME/.ix"

actual_sha=$(git -C "$REPO" rev-parse HEAD)
if [[ "$actual_sha" != "$RELEASE_SHA" ]]; then
  printf 'release SHA mismatch: expected %s, got %s\n' "$RELEASE_SHA" "$actual_sha" >&2
  exit 2
fi
for executable in "$LOCKED_BUILD" "$EA_BIN" "$QUIRE_BIN" "$IX_FLOW_RUNNER" "$FILAMENT_ANALYZE_BIN" "$PDFTOTEXT"; do
  if [[ ! -x "$executable" ]]; then
    printf 'required executable is missing: %s\n' "$executable" >&2
    exit 2
  fi
done

mkdir -p "$TMPDIR" "$TARGET_DIR" "$CLEAN_HOME"
export TMPDIR TZ=UTC IX_FLOW_RUNNER QUIRE_DRIVER_BIN="$QUIRE_BIN" QUIRE_E2E_QUIRE="$QUIRE_BIN"
export QUOIN_E2E_EA="$EA_BIN" FILAMENT_ANALYZE_BIN PDFTOTEXT OPENVEX_SCHEMA_PATH CYCLONEDX_SCHEMA_PATH
export SPEC_ARTIFACTS_PROCESS_DIR SPEC_OBJECTS_SAFETY_DIR SPEC_OBJECTS_SECURITY_DIR
export QUOIN_SACM_METAMODEL=/home/peter/dev/.scratchpad/OMG-SACM-2.3-SACM.xml
export PATH="$IX_FLOW_BIN:/home/peter/.local/bin:/home/peter/.npm-global/bin:$PATH"

build_run() {
  local output status request_ref request_id output_dir
  if output=$(CARGO_TARGET_DIR="$TARGET_DIR" "$LOCKED_BUILD" run \
    --owner peter@terra:quoin:codex-release \
    --ticket PLAT-1822 \
    --class must-fix \
    --units 2 \
    -- "$@" 2>&1); then
    status=0
  else
    status=$?
  fi
  printf '%s\n' "$output"
  request_ref=$(printf '%s\n' "$output" | sed -n 's/^request=\([^ ]*\) output=\(.*\)$/\1 \2/p' | tail -1)
  if [[ -n "$request_ref" ]]; then
    read -r request_id output_dir <<<"$request_ref"
    printf '\n--- locked-build stdout: %s ---\n' "$request_id"
    cat "$output_dir/$request_id.stdout"
    printf '\n--- locked-build stderr: %s ---\n' "$request_id"
    cat "$output_dir/$request_id.stderr" >&2
  fi
  return "$status"
}

run_step() {
  local name="$1"
  shift
  local log="$RUN_DIR/quoin-v0.38.1-${RELEASE_SHORT}-${name}.log"
  local status_file="$RUN_DIR/quoin-v0.38.1-${RELEASE_SHORT}-${name}.status"
  printf 'running %s on %s\n' "$name" "$RELEASE_SHA"
  if "$@" >"$log" 2>&1; then
    printf '0\n' >"$status_file"
    printf 'PASS %s\n' "$name"
  else
    local status=$?
    printf '%s\n' "$status" >"$status_file"
    printf 'FAIL %s exit=%s log=%s\n' "$name" "$status" "$log" >&2
    tail -40 "$log" >&2
    return "$status"
  fi
}

cd "$RUST_DIR"
failures=0
run_step quoin-binary-build build_run cargo build --locked -j4 -p quoin-cli --bin quoin || exit $?
run_step defaults-seed "$QUOIN_BIN" module ensure-defaults || exit $?
if [[ "${1:-}" != "--remaining" ]]; then
  run_step nonjev-full-suite build_run cargo test --locked --workspace --exclude quoin-jev --tests -j4 -- --test-threads=4 || failures=1
fi
run_step catalog-module-targets build_run cargo test --locked -p quoin-catalog -p quoin-modules -j4 -- --test-threads=4 || failures=1
run_step cli-catalog-target build_run cargo test --locked -p quoin-cli --test tc_catalog_command -j4 -- --test-threads=4 || failures=1
run_step real-ea-quire-export build_run cargo test --locked -j4 -p quoin-cli --features e2e-release-export --test tc_191_real_release_export real_ea_release_and_quire_dossier_export_scenario -- --exact --test-threads=4 || failures=1

run_step clean-home-version env "HOME=$CLEAN_HOME" "IX_HOME=$CLEAN_IX_HOME" "$QUOIN_BIN" --version || failures=1
run_step clean-home-ensure-defaults env "HOME=$CLEAN_HOME" "IX_HOME=$CLEAN_IX_HOME" "$QUOIN_BIN" module ensure-defaults || failures=1
run_step clean-home-module-list env "HOME=$CLEAN_HOME" "IX_HOME=$CLEAN_IX_HOME" "$QUOIN_BIN" module list || failures=1
run_step clean-home-catalog-validate env "HOME=$CLEAN_HOME" "IX_HOME=$CLEAN_IX_HOME" "$QUOIN_BIN" catalog validate || failures=1
exit "$failures"
