#!/usr/bin/env bash
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
fixture_root="$repo_root/tests/fixtures/security-analysis-profile"
skill_dir="$repo_root/skills/spec-security-analysis"
state_file="$fixture_root/skill-run-state.json"

for name in IX_HOME IX_FILAMENT_MODULES_PATH QUOIN_SEMANTIC_ROOT QUOIN_ORG; do
  if [[ -z "${!name-}" ]]; then
    echo "set $name to the installed synthetic profile and authoring modules" >&2
    exit 2
  fi
done

run_root=$(mktemp -d)
trap 'python3 -c '\''import shutil,sys; shutil.rmtree(sys.argv[1], ignore_errors=True)'\'' "$run_root"' EXIT

for profile in a b; do
  run_repo="$run_root/$profile"
  cp -a "$fixture_root/$profile/repo" "$run_repo"
  git -C "$run_repo" init -q

  before_eval="$run_root/$profile-before-eval"
  before_context=$(cd "$run_repo" && quoin profile evaluate --out-dir "$before_eval" | jq -cS '.context')
  selection_before=$(base64 -w0 "$run_repo/.ix/source-profile.yaml")

  codex exec --ephemeral --sandbox workspace-write --cd "$run_repo" \
    --add-dir "$skill_dir" --add-dir "$IX_HOME" --add-dir "$IX_FILAMENT_MODULES_PATH" \
    "Read and follow the security-analysis skill at $skill_dir/SKILL.md. Run the profile evaluation, then author and validate only the control, scope_exclusion and SpecReview artifacts required by the skill. Use only the resulting source-profile-evaluation-v1 report. Map only binding clauses; do not create controls for not_binding, out-of-profile or unresolved clauses. Include unresolved clauses and missing dimensions in SpecReview. Do not change .ix/source-profile.yaml or any context value. The profile and all available clause sets are synthetic test fixture data; no other standards or clauses are available. Validate all produced artifacts before finishing."

  selection_after=$(base64 -w0 "$run_repo/.ix/source-profile.yaml")
  after_eval="$run_root/$profile-after-eval"
  after_context=$(cd "$run_repo" && quoin profile evaluate --out-dir "$after_eval" | jq -cS '.context')

  python3 - "$state_file" "$profile" "$selection_before" "$selection_after" "$before_context" "$after_context" <<'PY'
import base64
import json
import pathlib
import sys

path, name, before_selection, after_selection, before_context, after_context = sys.argv[1:]
state = json.loads(pathlib.Path(path).read_text()) if pathlib.Path(path).exists() else {"fixtures": {}}
state["fixtures"][name] = {
    "runner": "codex exec",
    "selection_before": base64.b64decode(before_selection).decode(),
    "selection_after": base64.b64decode(after_selection).decode(),
    "context_before": json.loads(before_context),
    "context_after": json.loads(after_context),
}
pathlib.Path(path).write_text(json.dumps(state, indent=2, sort_keys=True) + "\n")
if before_selection != after_selection or before_context != after_context:
    raise SystemExit(f"fixture {name} changed selection or evaluated context during skill run")
PY

  for dir_pair in "security:spec/security" "reviews:reviews"; do
    dest_name=${dir_pair%%:*}
    source_name=${dir_pair#*:}
    if [[ -d "$run_repo/$source_name" ]]; then
      python3 - "$fixture_root/$profile/output/$dest_name" "$run_repo/$source_name" <<'PY'
import pathlib
import shutil
import sys

destination, source = map(pathlib.Path, sys.argv[1:])
shutil.rmtree(destination, ignore_errors=True)
shutil.copytree(source, destination)
PY
    fi
  done
done
