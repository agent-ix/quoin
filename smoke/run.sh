#!/usr/bin/env bash
#
# Host-side driver: build the clean-room image and run the smoke test.
#
# Mounts the host's Claude subscription credential read-only when present (never
# read/extracted — only the file is bind-mounted, so the live "plugin loaded"
# check uses your subscription with no API key). Without it, the test still runs
# filesystem-only checks.
#
# Env knobs:
#   PKG_VERSION     exact published version to install (default pinned below)
#   CLAUDE_VERSION  exact Claude Code version in the image (default pinned below)
#   PLUGIN_SOURCE   github (default; real community path) | local (this checkout)
#
# Examples:
#   make install-smoke
#   PLUGIN_SOURCE=local ./smoke/run.sh          # validate the working tree (CI)
#   PKG_VERSION=0.2.1 ./smoke/run.sh            # pin for reproducibility
set -euo pipefail

SMOKE_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SMOKE_DIR/.." && pwd)"
IMAGE="quoin-install-smoke"
PKG_VERSION="${PKG_VERSION:-0.22.5}"
CLAUDE_VERSION="${CLAUDE_VERSION:-2.1.220}"
CODEX_VERSION="${CODEX_VERSION:-0.149.1}"
PLUGIN_SOURCE="${PLUGIN_SOURCE:-github}"

echo ">> build $IMAGE (CLAUDE_VERSION=$CLAUDE_VERSION CODEX_VERSION=$CODEX_VERSION)"
docker build -t "$IMAGE" \
  --build-arg CLAUDE_VERSION="$CLAUDE_VERSION" \
  --build-arg CODEX_VERSION="$CODEX_VERSION" "$SMOKE_DIR"

args=(--rm -e PKG_VERSION="$PKG_VERSION" -e PLUGIN_SOURCE="$PLUGIN_SOURCE")

# gh skill install (opencode / GitHub Copilot stages) authenticates with GH_TOKEN.
GH_TOKEN_VALUE="${GH_TOKEN:-${GITHUB_TOKEN:-$(gh auth token 2>/dev/null || true)}}"
if [ -n "$GH_TOKEN_VALUE" ]; then
  args+=(-e GH_TOKEN="$GH_TOKEN_VALUE")
  echo ">> passing GH_TOKEN for gh skill install"
else
  echo ">> WARN: no GH token (gh skill install for opencode/Copilot may fail or rate-limit)"
fi

if [ "$PLUGIN_SOURCE" = "local" ]; then
  args+=(-v "$REPO_ROOT:/repo:ro")
  echo ">> plugin source: local checkout ($REPO_ROOT)"
else
  echo ">> plugin source: github:agent-ix (real community path)"
fi

creds="$HOME/.claude/.credentials.json"
if [ -f "$creds" ]; then
  args+=(-v "$creds:/seed/.credentials.json:ro")
  [ -f "$HOME/.claude.json" ] && args+=(-v "$HOME/.claude.json:/seed/.claude.json:ro")
  echo ">> mounting host subscription credential (live plugin-load check enabled)"
else
  echo ">> no host credential found — filesystem-only checks"
fi

echo ">> run smoke (PKG_VERSION=$PKG_VERSION)"
docker run "${args[@]}" "$IMAGE"
