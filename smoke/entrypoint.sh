#!/usr/bin/env bash
#
# Runs inside the clean-room container. Three stages, then assertions:
#   Stage 1   install the published CLI from PUBLIC npm (no auth, no .npmrc)
#   Stage 1b  resolve the default module set from an EMPTY IX_HOME and assert it
#             matches the shipped default-modules.yaml (see smoke/modules.mjs)
#   Stage 2   install the Claude Code plugin and its skills/workflows
#   assert    the CLI bin runs, the plugin installed, its skills/workflows exist
#
# PLUGIN_SOURCE selects where the plugin comes from:
#   github (default)  /plugin marketplace add agent-ix/<repo>  — the real
#                     community path, validating what's live on GitHub's default
#                     branch. Used by `make install-smoke`.
#   local             marketplace add /repo (the mounted checkout) — validates
#                     THIS revision's plugin manifest. Used by CI on PRs, where
#                     the fix may not have reached the default branch yet.
#
# Subscription auth (host ~/.claude/.credentials.json mounted at /seed) is
# OPTIONAL: plugin marketplace add/install are pure git + local cache and need
# no Anthropic auth, so the "skills present" checks are deterministic with or
# without creds. Creds only enable the bonus "Claude actually loaded the plugin
# and exposes its skills as commands" confirmation from a live session.
set -uo pipefail

: "${PKG:?PKG not set}"
: "${BIN:?BIN not set}"
: "${REPO:?REPO not set}"
: "${PLUGIN:?PLUGIN not set}"
: "${MARKETPLACE:?MARKETPLACE not set}"
PKG_VERSION="${PKG_VERSION:-0.22.5}"
PLUGIN_SOURCE="${PLUGIN_SOURCE:-github}"
PUBLIC_REGISTRY="https://registry.npmjs.org/"
export HOME="${HOME:-/root}"

mkdir -p "$HOME/.claude"

echo "=================================================================="
echo " Community install smoke test: $PKG  (plugin source: $PLUGIN_SOURCE)"
echo " clean room: $(node -v) / npm $(npm -v) / claude $(claude --version 2>/dev/null || echo '?')"
echo "=================================================================="

if [ -f "$HOME/.npmrc" ] || [ -f /etc/npmrc ]; then
  echo "WARN: an .npmrc exists in the image — clean-room assumption weakened"
fi

# --- Auth (optional): seed the mounted host subscription credential ---
LIVE=0
if [ -f /seed/.credentials.json ]; then
  cp /seed/.credentials.json "$HOME/.claude/.credentials.json"
  chmod 600 "$HOME/.claude/.credentials.json"
  [ -f /seed/.claude.json ] && cp /seed/.claude.json "$HOME/.claude.json" || echo '{}' > "$HOME/.claude.json"
  LIVE=1
  echo "auth: seeded host subscription credential (live plugin-load check enabled)"
else
  echo '{}' > "$HOME/.claude.json"
  echo "auth: no credential mounted (filesystem-only checks; live load check skipped)"
fi

# ======================================================================
echo
echo "## Stage 1 — install published CLI from PUBLIC npm"
echo "   npm install -g $PKG@$PKG_VERSION  (registry: $PUBLIC_REGISTRY)"
if ! npm install -g "$PKG@$PKG_VERSION" --registry="$PUBLIC_REGISTRY"; then
  echo "FAIL: public npm install of $PKG failed"
  exit 1
fi
RESOLVED="$(npm ls -g "$PKG" --depth=0 2>/dev/null | grep -oE "${PKG}@[0-9][^ ]*" | head -1)"
echo "   resolved: ${RESOLVED:-unknown}"
if "$BIN" --help >/dev/null 2>&1; then
  echo "   PASS  '$BIN --help' runs"
else
  echo "   FAIL  '$BIN --help' did not run:"; "$BIN" --help || true
  exit 1
fi

# ======================================================================
# Stage 1b — the default module set, resolved from an EMPTY IX_HOME. A developer
# machine always has these installed already, so this is the only place the
# manifest's pins are checked the way a new user experiences them.
node /smoke/modules.mjs; modules_rc=$?

# ======================================================================
echo
echo "## Stage 2 — install Claude Code plugin"
case "$PLUGIN_SOURCE" in
  local)
    MARKET_SRC=/repo
    if [ ! -f /repo/.claude-plugin/marketplace.json ]; then
      echo "   FAIL  PLUGIN_SOURCE=local but /repo/.claude-plugin/marketplace.json is missing"
      echo "         (mount the repo checkout at /repo, or this repo lacks a marketplace manifest)"
      exit 1
    fi
    ;;
  github) MARKET_SRC="$REPO" ;;
  *) echo "   FAIL  unknown PLUGIN_SOURCE=$PLUGIN_SOURCE"; exit 2 ;;
esac
echo "   claude plugin marketplace add $MARKET_SRC"
claude plugin marketplace add "$MARKET_SRC" || echo "   (marketplace add reported non-zero — see assertions)"
echo "   claude plugin install $PLUGIN@$MARKETPLACE"
claude plugin install "$PLUGIN@$MARKETPLACE" || echo "   (plugin install reported non-zero — see assertions)"

# Capture the session init event. It lists loaded plugins[], plugin_errors, and
# every skill exposed as a slash command (e.g. "ix-flow:ix-flow") — the
# user-visible "installed AND usable" surface. init is emitted before
# authentication, so this works WITHOUT credentials and gives CI the same
# deterministic signal (the turn itself may then fail auth; we only need init).
STREAM=/tmp/init.stream.jsonl
: > "$STREAM"
timeout 180 claude -p "Reply with the single word: ok" \
  --output-format stream-json --verbose \
  </dev/null >"$STREAM" 2>/tmp/claude.err \
  || echo "   note: claude print-turn exited non-zero (init event still captured for assertions)"

# ======================================================================
/smoke/assert.sh "$STREAM"; claude_rc=$?

# ======================================================================
# Stage 3 — the additional coding agents (Codex / opencode / GitHub Copilot).
# Filesystem-install depth: each agent's documented install path runs and the
# bundle's skills must land on disk. See smoke/agents.sh.
/smoke/agents.sh; agents_rc=$?

echo
echo "=================================================================="
echo " Summary:  modules stage rc=$modules_rc   Claude Code stage rc=$claude_rc   other-agents stage rc=$agents_rc"
echo "=================================================================="
[ "$modules_rc" -eq 0 ] && [ "$claude_rc" -eq 0 ] && [ "$agents_rc" -eq 0 ]
