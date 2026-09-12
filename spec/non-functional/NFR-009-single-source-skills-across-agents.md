---
id: NFR-009
title: "Skills are authored once and consumed by every supported agent"
type: NFR
quality_attribute: portability
relationships:
  - target: "ix://agent-ix/quoin/US-009"
    type: "constrains"
  - target: "ix://agent-ix/quoin/StR-001"
    type: "traces_to"
---

# NFR-009: Skills are authored once and consumed by every supported agent

## Statement

quoin SHALL maintain a single skill tree under `skills/<name>/SKILL.md` as the
sole source of skill content, and each supported coding agent (Claude Code,
OpenAI Codex, opencode, GitHub Copilot) SHALL install from that one tree. Per-agent
plugin and marketplace manifests SHALL add only discovery and install metadata and
SHALL NOT contain a duplicated copy of any skill body.

## Scope

- Applies to: the `skills/` tree and the per-agent manifests that reference it
  (`.claude-plugin/`, `.codex-plugin/`, `.agents/plugins/marketplace.json`,
  `.github/plugin/marketplace.json`).
- Operational context: installing quoin as a plugin from any supported agent.

## Rationale

A single skill tree is what lets quoin add agents without forking content: every
agent reads the same `SKILL.md` files, so behavior stays identical and there is no
second copy to drift. Manifests differ per agent only in how they point at that
tree, keeping the maintenance cost of each additional agent close to one small
manifest.

## Measurement and Evaluation

| Metric                                                              | Target | Threshold | Method     |
| ------------------------------------------------------------------- | ------ | --------- | ---------- |
| Copies of any skill body outside the single `skills/` tree          | 0      | 0         | Inspection |
| Per-agent manifests that point at a skill tree other than `skills/` | 0      | 0         | Inspection |

## Verification

The repository is inspected to confirm exactly one `skills/` tree holds all
`SKILL.md` bodies, and each per-agent manifest (`.claude-plugin/`,
`.codex-plugin/plugin.json`, and the Codex/Copilot marketplace files) resolves its
skills to that same tree rather than carrying duplicated content.

## Dependencies

- **Upstream**: [US-009](../usecase/US-009-install-in-any-coding-agent.md) install
  quoin in the coding agent of choice.
- **Downstream**: the single-source rule governs how any future agent's manifest is
  added.
