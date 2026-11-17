---
id: SR-20261116075227
title: "Code review — PLAT-1630 Quoin native release smoke override"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; PLAT-1630 PR #970: .github/workflows/native-release.yml, .github/workflows/native-release-build.yml"
review_set: subset
---

## Summary

I reviewed PR #970's dispatch input and reusable native-release workflow changes. The package validation and checker setup remain hard failures, but the explicit override can waive any ix-flow smoke failure for any tag, not just the recorded clean-main case.

## Verdict

**CONDITIONAL** — constrain the exception to the recorded baseline failure and release before enabling it; the current boolean turns the entire smoke step into a bypass for arbitrary failures.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The override can hide unrelated ix-flow smoke failures for any release tag. | .github/workflows/native-release-build.yml:137-146; .github/workflows/native-release.yml:10-27 |

## Scope examined

- `.github/workflows/native-release.yml`: the workflow-dispatch input defaults to false and is passed to the reusable workflow.
- `.github/workflows/native-release-build.yml`: the reusable input defaults to false; packaged binary version, default-module setup, and catalog validation remain in a hard-failing step; npm and checker installation/build remain in a hard-failing preparation step; only the complete ix-flow smoke command has conditional `continue-on-error`.
- PLAT-1630 AC-5 as context: release output must be consumable by the EU pack, but it does not narrow the workflow exception to one known failure or tag.

## Review evidence

The frozen diff is at `d2fe22adebe3920e7be6035be94130faa742b242`. `git diff --check` passed and both changed workflow files parsed as YAML. No release test was rerun during this review. The recorded v0.36.0 run shows the binary version, default-module installation, and catalog validation succeeded before `smoke-formalize-success` failed; the new boolean allows any failure from that smoke command to be treated as non-blocking.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit 9aefc66f requires the explicit override, tag v0.36.0, and all three recorded clean-main markers before it returns success; every other nonzero smoke status is returned unchanged. |

## Disposition evidence

The only bypass path checks the explicit true input, exact tag `v0.36.0`, and the exact smoke scenario, invariant, and missing-field list. The command's captured nonzero status is returned if any condition fails. Version/catalog validation and workflow-checker preparation remain separate hard-failing steps.

```bash
if [ "$ALLOW_KNOWN_WORKFLOW_SMOKE_FAILURE" = "true" ] \
  && [ "$TAG" = "v0.36.0" ] \
  && grep -Fq 'smoke-formalize-success' "$SMOKE_LOG" \
  && grep -Fq 'invariant_interview_incomplete' "$SMOKE_LOG" \
  && grep -Fq '"missing":["engineering_assurance","allowed_signers"]' "$SMOKE_LOG"; then
  echo "::warning::Allowing the recorded v0.36.0 clean-main smoke-formalize-success failure; all other smoke failures remain fatal."
  exit 0
fi
exit "$SMOKE_STATUS"
```
