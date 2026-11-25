#!/bin/sh
# Trace: FR-226-AC-11
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
model=${QUOIN_SACM_METAMODEL:?QUOIN_SACM_METAMODEL must name the external OMG SACM 2.3 metamodel XML}
case "$model" in
  /*) ;;
  *) model="$repo_root/$model" ;;
esac

cd "$repo_root/rust"
QUOIN_SACM_METAMODEL="$model" cargo test --locked \
  --target-dir "${QUOIN_CARGO_TARGET:-$repo_root/rust/target}" \
  -p quoin-assurance --test tc_sacm_export sacm_uses_normative_metamodel -- --ignored
