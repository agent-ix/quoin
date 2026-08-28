# syntax=docker/dockerfile:1
#
# Clean-room "community install" smoke image for @agent-ix/quoin.
#
# The build context is THIS directory (smoke/), which deliberately contains no
# .npmrc and no @agent-ix scope override — so the container resolves packages
# exactly like an outside developer on a fresh machine would.
#
# The package + Claude Code plugin are installed at RUNTIME (entrypoint), not at
# build time, so bumping PKG_VERSION needs no rebuild.
FROM node:24-bookworm-slim@sha256:ba849c60be29959425b8734d57b8b4b7d56f98edd9504c9af091d5281095a71e

ARG CLAUDE_VERSION=2.1.220
ARG CODEX_VERSION=0.149.1

# Base tools + the GitHub CLI from GitHub's official apt repo (>= 2.90 for
# `gh skill`, which installs the opencode/Copilot skill bundles). Claude Code
# and the Codex CLI are installed globally; opencode and Copilot need no host
# CLI here — their install path is `gh skill install`.
RUN apt-get update \
 && apt-get install -y --no-install-recommends git jq ca-certificates wget \
 && mkdir -p -m 755 /etc/apt/keyrings \
 && wget -nv -O/tmp/ghkey.gpg https://cli.github.com/packages/githubcli-archive-keyring.gpg \
 && cat /tmp/ghkey.gpg | tee /etc/apt/keyrings/githubcli-archive-keyring.gpg >/dev/null \
 && chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg \
 && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" > /etc/apt/sources.list.d/github-cli.list \
 && apt-get update \
 && apt-get install -y --no-install-recommends gh \
 && rm -rf /var/lib/apt/lists/* \
 && npm install -g @anthropic-ai/claude-code@${CLAUDE_VERSION} @openai/codex@${CODEX_VERSION}

# Per-package identity consumed by the generic entrypoint/assert scripts.
ENV PKG=@agent-ix/quoin \
    BIN=quoin \
    PLUGIN=quoin \
    MARKETPLACE=quoin \
    REPO=agent-ix/quoin \
    HOME=/root

COPY . /smoke
RUN chmod +x /smoke/entrypoint.sh /smoke/assert.sh /smoke/agents.sh /smoke/run.sh

ENTRYPOINT ["/smoke/entrypoint.sh"]
