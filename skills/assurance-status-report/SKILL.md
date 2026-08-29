---
name: assurance-status-report
description: Produce an evidence-backed status report for a Quoin/Quire epic, phase, or assurance campaign. Use when asked for progress, percent complete, before/after metrics, completed-ticket inventory, open-work classification, or readiness for the next phase. The workflow is read-only and does not comment on, close, or modify tickets or repositories.
---

# Assurance Status Report

Report what the retained evidence proves about a named goal. Use Quoin for
measurement meaning and history, Quire for producer provenance, git for local
state, and GitHub for current ticket and promotion state. Do not recreate their
analysis in the skill.

Read [references/report-contract.md](references/report-contract.md) before
assembling the report.

## Boundary

This workflow is read-only. It may inspect repositories, retained evidence, and
remote ticket state. It must not run measurement producers, update baselines,
modify evidence, push branches, or create/comment/close/reopen issues or pull
requests unless the user separately authorizes that mutation.

A closed ticket is workflow state, not proof of completion. Match each claimed
result to its acceptance evidence, promoted commit, and relevant current-state
check.

## Inputs

Establish these before collecting evidence:

- the goal, epic, phase, or campaign being reported;
- repositories and tickets in scope;
- work the user explicitly excluded;
- the baseline and current endpoint; and
- whether the user also wants an intermediate phase boundary.

If no baseline is named, use the earliest retained collection explicitly linked
by the target epic. Use the latest compatible retained collection as current.
State that selection instead of silently choosing favorable snapshots.

## Collect exact evidence

### Repository state

For every participating repository, record:

```bash
git rev-parse HEAD
git status --porcelain
git remote get-url origin
```

Resolve revisions to full 40-character SHAs. Use the canonical remote and a
versioned, read-only GitHub query to confirm current default-branch and commit
reachability, for example:

```bash
gh api --header 'X-GitHub-Api-Version: 2022-11-28' \
  repos/OWNER/REPO/commits/FULL_SHA
```

Validate required response fields rather than assuming a successful command
returned the requested object. Record dirty, unreachable, ambiguous, or
wrong-origin state as a limitation; never normalize it to clean.

### Tool and measurement state

Use existing machine-readable interfaces:

```bash
quoin report --format json
quoin report --series METRIC --format json
quoin report --since FULL_BASELINE_REVISION --format json
quire provenance --json
```

Use `quoin report` to discover active MeasurementPlans and retained values. Use
the series or comparison view for historical values; do not scrape rendered
human output when JSON exists. Confirm Quire's provenance schema, full CLI and
engine revisions, clean state, and required capabilities before attributing a
measurement to it.

Record the exact collection id, timestamp, source revision, corpus revision,
tool identity/version, configuration digest, definition version, and population
identity supporting each reported value. If the repository has a verification
stack attestation, report its lock and executable digests.

Do not regenerate evidence merely to answer a status question. A new run changes
the evidence state and requires separate authorization.

### Ticket and promotion state

Use versioned read-only GitHub REST or GraphQL queries. For each issue capture at
least number, title, state, URL, and closed time. For each delivery pull request
capture state, base/head branches, merge time, merge commit, and URL. Confirm the
current default-branch SHA separately.

Follow relationships from the target epic and its retained implementation
comments. Do not equate “mentioned nearby” with “child.” Classify every remaining
item as blocking, non-blocking, downstream, excluded, or unknown and state the
evidence for that disposition.

## Interpret without hiding drift

Define every project-specific term before its first use in the metrics section.
Derive definitions from the active MeasurementPlan or other versioned contract.
At minimum define any use of finding, controlled case, differential pair,
localization, L1/L2/L3, actionability, span grounding, safe refusal, Tier 1,
Tier 2, ratchet, drift mutation, comparability, or fail closed.

For every percentage, report its numerator and denominator. Express rate changes
as percentage points. Compare baseline and current numerically only when their
definition versions and population identities are compatible. If either moved,
show both values and explain the change, but mark the delta incomparable.

Keep historical metrics visible when a stronger successor exists. Explain why
the successor is more decision-relevant; do not relabel old evidence under the
new definition.

If the user asks for percent complete, use authored task weights when present.
Otherwise report the unweighted count of proven-complete in-scope tasks and say
that this is the method. Present verification and promotion gates separately so
a high task percentage cannot conceal an unverified result.

## Report and stop

Follow the ordering in the report contract: status, definitions, metrics,
completed work, open work, drift controls, limitations, and progress judgment.
Lead with whether the named goal is complete, progressing, at risk, blocked, or
not started, but put the glossary before the first detailed metric use.

Link to the exact retained collections, issues, pull requests, and promoted
commits. Distinguish evidence observed now from historical evidence retained by
the campaign. If required evidence is missing or incompatible, say
`unknown` or `not_computed` and identify what would resolve it.
