---
id: SR-20261112024909
title: "Criterion-strength review of quoin PR 830 (FR-137..139)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/quoin@3157f4ba; spec/functional/FR-137-detect-and-place-licensed-packs.md, spec/functional/FR-138-never-publish-an-installed-pack.md, spec/functional/FR-139-pack-writes-nothing-into-a-repository.md"
review_set: subset
---

# Criterion-strength review of quoin PR 830

## Summary

Ticket: PLAT-1486. Judged each of the 23 ACs for whether a test oracle exists and can fail.
Jev was not used; this is manual judgment over the spec text and the quoin source at 3157f4ba.
FR-139's no-writes oracle is strong: AC-3 snapshots the whole sandbox outside `IX_HOME`, and
AC-6 is a control that proves the oracle can fail. FR-137-AC-1..5, 7, 8 and FR-138-AC-1, 4, 5, 7
are concrete and can fail.

## Verdict

**FAIL, two high, one medium.** FR-138's publish-path ACs have nothing to run against, and the
static-token oracles cannot pass as written.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-138-AC-2 and AC-3 have no subject. quoin has no command that packages, uploads or lists a module for distribution (no such function in rust/crates), so "a publish path invoked over a set" cannot be run and the ACs pass vacuously. Name a concrete existing command, or drop AC-2/AC-3 and keep the guard as a library function tested by AC-1 and AC-7 plus its one real caller, `ensure-defaults` (AC-4) | spec/functional/FR-138-never-publish-an-installed-pack.md:50-51 |
| FND-002 | high | FR-138-AC-8 is not enumerable. "Every function that writes module content outside the modules directory appears in a checked list" is a hand-kept inventory, and no test can detect "a writer that is not in the list", because nothing defines which functions write module content. A hand-kept list is file tracking, which the repo rules forbid. Delete AC-8 and its Behavior clause; keep one guard function and test its callers by behaviour | spec/functional/FR-138-never-publish-an-installed-pack.md:56 |
| FND-003 | medium | The static-token oracles contradict each other and cannot pass. FR-137-AC-9 bans the token `text` (it is everywhere). FR-138-AC-6 bans `licensed` and `clearance_ref` in "quoin source", but the required fixtures (`text: licensed`) live in quoin's test tree. Behavior FR-138:42 bans comparison "against the `rights` field", a third token set. Replace with one inspection scoped to the non-test `src/` of the named crates, with one token set, or rely on the behavioural flip (FR-137-AC-6, FR-138-AC-7) and drop the grep | spec/functional/FR-137-detect-and-place-licensed-packs.md:67, spec/functional/FR-138-never-publish-an-installed-pack.md:42, spec/functional/FR-138-never-publish-an-installed-pack.md:54 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-137-AC-1..9, path: spec/functional/FR-137-detect-and-place-licensed-packs.md, role: examined, excerpt: "lines 59-67"}
- {id: FR-138-AC-1..8, path: spec/functional/FR-138-never-publish-an-installed-pack.md, role: examined, excerpt: "lines 49-56"}
- {id: FR-139-AC-1..6, path: spec/functional/FR-139-pack-writes-nothing-into-a-repository.md, role: examined, excerpt: "lines 38-43; AC-6 control proves the AC-3 snapshot oracle can fail"}
```
