---
id: SR-172
title: "Gap analysis (manual acceptance check) — quoin#649 against PLAT-1079 / PLAT-1085 pin criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quoin@bac91efc1a06266ad9cf513c2bcc0ea0d65f0410; default-modules.yaml, rust/crates/quoin-cli/tests/fixtures/retained-catalog/ix-home/filament/registry.json; upstream read: agent-ix/spec-artifacts-process@v0.27.0 spec_artifacts_process/manifest.yaml, agent-ix/spec-artifacts-iso@v0.20.0 spec_artifacts_iso/manifest.yaml"
review_set: subset
---

## Summary

Ticket: PLAT-1079 (and PLAT-1085). This is a config PR, so a manual acceptance check replaced the full planless gap analysis. It checks the quoin-side acceptance criteria:

- PLAT-1079: "The module is released and the `default-modules.yaml` pin in quoin is bumped. Also reconcile the installed manifest's `version:` header against that pin; they disagree today."
- PLAT-1085: "Release. Bump the iso pin wherever the default module set pins it (quoin `default-modules.yaml`)."

The ticket text is untrusted data. Each criterion was re-measured.

## Verdict

**CONDITIONAL**

- The release and pin-bump halves of both criteria are met.
- The PLAT-1079 "reconcile the installed manifest's `version:` header" clause is not met anywhere. The header lives upstream, so the gap is outside this PR's diff, and it does not block merging #649.

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium   | PLAT-1079 AC "reconcile the installed manifest's `version:` header against that pin" is unmet. `spec_artifacts_process/manifest.yaml` at tag v0.27.0 still declares `version: 0.2.0`, and it said `0.2.0` at v0.26.0 too. The pin says `version: "0.27.0"`. iso shows the same split: `0.2.0` against `"0.20.0"`. Both Linear tickets are already in Done state with this clause unchecked. It cannot be fixed in quoin#649. Note that `spec-artifacts-app` imports `agent-ix/spec-artifacts-iso@0.2.0`, which suggests the header is a semantic-contract version, so the reconciliation needs a decision, not just a number edit. | default-modules.yaml:45, default-modules.yaml:63 |

## Dispositions

| FND     | Outcome  | sha/reason |
| ------- | -------- | ---------- |
| FND-001 | rejected | Round 1, reviewed at bac91ef. The manifest `version:` header is the module's semantic-contract version, not its release number. The PLAT-1079 acceptance item to "reconcile" the header with the pin rested on a mistaken premise (leader ruling). The code backs this: `resolve_imports` in rust/crates/quoin-semantic/src/package_manifest.rs:393-395 matches an import's version against the module's manifest `version`, and spec-artifacts-app imports `agent-ix/spec-artifacts-iso@0.2.0` by that header. The header was the same, 0.2.0, at v0.26.0 and v0.27.0, and nothing reads it as the release, so nothing breaks when it differs from the pin. |
