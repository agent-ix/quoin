#!/usr/bin/env bash
#
# Stage 3 — clean-room install check for the NON-Claude coding agents.
#
# For each additional agent quoin/ix-flow supports as a first-class plugin, run
# that agent's DOCUMENTED install path and assert every `skill <name>` line in
# expected.txt lands as a SKILL.md on disk. Depth is intentionally
# filesystem-level (install command succeeds + skill files present) — the
# cross-agent analog of the "skills present" half of the Claude check in
# assert.sh. The richer "loaded and usable as a /command" assertion stays
# Claude-specific (each agent would need its own credentialed live session).
#
#   OpenAI Codex      codex plugin marketplace add <src> + codex plugin add
#                     <plugin>@<marketplace> — exercises THIS repo's
#                     .codex-plugin/plugin.json + .agents/plugins/marketplace.json.
#                     Marketplace add/plugin add are git + local cache (no OpenAI
#                     auth needed).
#   opencode          gh skill install <src> --agent opencode
#   GitHub Copilot    gh skill install <src> --agent github-copilot
#                     gh skill reads the repo's skills/ tree; auth via GH_TOKEN.
#
# PLUGIN_SOURCE (shared with Stage 2): github → install from agent-ix/<repo>'s
# default branch (validates what is LIVE; the Codex marketplace manifest must be
# pushed first); local → install from the mounted /repo checkout (validates THIS
# revision, the CI/PR path).
set -uo pipefail

: "${REPO:?REPO not set}"
: "${PLUGIN:?PLUGIN not set}"
: "${MARKETPLACE:?MARKETPLACE not set}"
PLUGIN_SOURCE="${PLUGIN_SOURCE:-github}"
SKILL_PIN="${SKILL_PIN:-}"
export HOME="${HOME:-/root}"
fail=0

SMOKE_DIR="$(cd "$(dirname "$0")" && pwd)"
EXPECTED="$SMOKE_DIR/expected.txt"

# The bundle's user-visible skills (the `skill <name>` lines in expected.txt).
SKILLS=()
while read -r kind name _; do
  [ "${kind:-}" = "skill" ] && SKILLS+=("$name")
done < "$EXPECTED"

# Codex validates THIS repo's new manifest (.codex-plugin + .agents marketplace),
# so its source honors PLUGIN_SOURCE: local → the mounted /repo checkout (this
# revision); github → agent-ix/<repo>'s default branch (must be pushed first).
case "$PLUGIN_SOURCE" in
  github) CODEX_SRC="$REPO" ;;
  local)
    CODEX_SRC=/repo
    if [ ! -f /repo/.agents/plugins/marketplace.json ]; then
      echo "   FAIL  PLUGIN_SOURCE=local but /repo/.agents/plugins/marketplace.json is missing"
      exit 1
    fi
    ;;
  *) echo "   FAIL  unknown PLUGIN_SOURCE=$PLUGIN_SOURCE"; exit 2 ;;
esac

# opencode & GitHub Copilot add no repo-specific files. They use the remote
# repository because `--from-local` recursively re-discovers nested workflow
# assets as duplicate skill names. In local/CI mode, `--pin` binds that remote
# read to the exact mounted checkout commit instead of silently testing the
# default branch.
GH_SRC="$REPO"
GH_PIN_ARGS=()
if [ -n "$SKILL_PIN" ]; then
  GH_PIN_ARGS=(--pin "$SKILL_PIN")
fi

# assert every expected skill has a SKILL.md somewhere under $1
assert_skills_in() {
  local root="$1" agent="$2" miss=0 s
  for s in "${SKILLS[@]}"; do
    if [ -n "$(find "$root" -path "*/$s/SKILL.md" 2>/dev/null | head -1)" ]; then
      echo "   PASS  [$agent] skill present: $s"
    else
      echo "   FAIL  [$agent] skill MISSING: $s"; miss=1
    fi
  done
  return "$miss"
}

# --- OpenAI Codex (native marketplace — validates the .codex-plugin manifest) ---
echo
echo "## Stage 3a — OpenAI Codex   (codex plugin marketplace add $CODEX_SRC; plugin add $PLUGIN@$MARKETPLACE)"
if codex plugin marketplace add "$CODEX_SRC" </dev/null >/tmp/codex.log 2>&1 \
   && codex plugin add "$PLUGIN@$MARKETPLACE" </dev/null >>/tmp/codex.log 2>&1; then
  assert_skills_in "$HOME/.codex/plugins/cache" codex || fail=1
else
  echo "   FAIL  [codex] install failed:"; sed 's/^/         /' /tmp/codex.log | tail -20; fail=1
fi

# opencode & GitHub Copilot install via `gh skill`, which authenticates with
# GH_TOKEN. When no token is available (e.g. a contributor without `gh auth`),
# SKIP these two stages with a warning rather than fail — the analog of the
# optional Claude subscription credential. CI passes GITHUB_TOKEN so they assert.
if [ -n "${GH_TOKEN:-}" ] || gh auth status >/dev/null 2>&1; then

  # --- opencode (gh skill — the documented opencode path) ---
  echo
  echo "## Stage 3b — opencode        (gh skill install $GH_SRC --all --agent opencode)"
  rm -rf /tmp/smoke-opencode
  if gh skill install "$GH_SRC" --all "${GH_PIN_ARGS[@]}" --agent opencode --dir /tmp/smoke-opencode >/tmp/opencode.log 2>&1; then
    assert_skills_in /tmp/smoke-opencode opencode || fail=1
  else
    echo "   FAIL  [opencode] gh skill install failed:"; sed 's/^/         /' /tmp/opencode.log | tail -20; fail=1
  fi

  # --- GitHub Copilot (gh skill — the documented Copilot CLI / gh path) ---
  echo
  echo "## Stage 3c — GitHub Copilot  (gh skill install $GH_SRC --all --agent github-copilot)"
  rm -rf /tmp/smoke-copilot
  if gh skill install "$GH_SRC" --all "${GH_PIN_ARGS[@]}" --agent github-copilot --dir /tmp/smoke-copilot >/tmp/copilot.log 2>&1; then
    assert_skills_in /tmp/smoke-copilot github-copilot || fail=1
  else
    echo "   FAIL  [github-copilot] gh skill install failed:"; sed 's/^/         /' /tmp/copilot.log | tail -20; fail=1
  fi

else
  echo
  echo "## Stage 3b/3c — opencode & GitHub Copilot: SKIPPED"
  echo "   no GH token (set GH_TOKEN or run \`gh auth login\`); \`gh skill install\` needs auth"
fi

echo
if [ "$fail" -eq 0 ]; then
  echo "## STAGE 3 RESULT: PASS — Codex, opencode, and GitHub Copilot install the bundle's skills"
else
  echo "## STAGE 3 RESULT: FAIL — see misses above"
fi
exit "$fail"
