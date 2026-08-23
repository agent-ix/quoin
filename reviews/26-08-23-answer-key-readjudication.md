---
id: SR-020
title: "Re-adjudication — the tier-2 answer key against the code as it stands (quoin#200)"
type: SpecReview
analysis: gap-analysis
scope: "bench/answer-key.json, tests/bench-corpora.test.ts, spec/matrix.md"
review_set: subset
---

# SR-020: Re-adjudication — the tier-2 answer key against the code as it stands (quoin#200)

## Summary

Re-adjudicated all seven entries of `bench/answer-key.json` by locating each
detector **and its caller**, not by re-reading the notes. Two `now_detectable`
claims were wrong, both in the direction that flatters the toolchain; one family
had never had a tracking ticket of its own; and every entry was carrying a
boolean where the honest answer needs three values.

The key is the recall denominator for "would the tools have found what humans
found". Every error corrected here made that denominator read better than the
truth.

## Verdict

**CONDITIONAL** — no `high` findings. FND-001 and FND-002 are corrections to a
published artifact rather than defects in shipped behaviour, and both are now
pinned by tests that fail if the state is restored.

## Findings

| ID      | Severity | Summary                                                                                                            | Refs                  |
| ------- | -------- | ------------------------------------------------------------------------------------------------------------------ | --------------------- |
| FND-001 | medium   | AK-005 claimed `now_detectable: "partially"` with a date and a fixing PR for a detector with no caller — corrected | bench/answer-key.json |
| FND-002 | medium   | AK-006's `now_detectable: false` was right for a reason nobody had re-checked since the detector shipped           | bench/answer-key.json |
| FND-003 | medium   | AK-007 pointed at quoin#204 — another family's ticket — so `gate-that-gates-nothing` had no owner                  | bench/answer-key.json |
| FND-004 | low      | `now_detectable` is a boolean over a three-valued fact: located, aggregate, none                                   | bench/answer-key.json |

## Detail

### FND-001 — three fields, all wrong, all in the same direction

AK-005 (`oracle-is-code-copy`) carried `now_detectable: "partially"`,
`detectable_since: "quire-rs main, post v0.43.0"`, and `fixed_by:
"agent-ix/quire-rs#247"`.

`grep -rn oracle_copies src/` in quire-rs returns the definition and two unit
tests. **There is no production caller.** The similarity comparison ships and is
tested; the join from an oracle span to the implementation it judges needs the
`Registry` and the `implements` relation, which the binder does not hold. The
tier-1 `oracle-copy` corpus confirms it: the engine reports nothing at all.

`partially` reads as "some of it fires", and none of it does. Now
`now_detectable: false`, `detection_strength: "none"`, `detectable_since: null`,
`fixed_by: null`, `tracked_by: agent-ix/quire-rs#236` — a capability nothing can
reach has no date it became available and no PR that delivered it.

That AK-005 was pass 2's **highest-value finding** and the key credited it as
partly mechanised is the sharpest version of this programme's premise.

### FND-002 — right answer, stale reason

AK-006 (`mocked-confirmation`) said `now_detectable: false`. Correct — and the
recorded reason was "no mechanized detector yet", written before quoin#204
shipped one, and never re-checked after.

The detector exists at `src/auditor/audit.ts:241` and **still cannot fire**.
`mockedBindings` opens with `if (injections.length === 0) return []`, and
`AuditInput.injections` is optional, supplied by none of the three production
`audit()` callers, and produced by nothing in the tree. The 8 unit tests build
`injections` inline, which is how a green suite sat over a dead path.

The plan for this re-adjudication predicted the opposite — that AK-006
_under-counted_ a capability that existed. Checking rather than assuming is what
turned that around. quoin#204 reopened.

### FND-003 — a family with no owner

AK-007 (`gate-that-gates-nothing`) carried `tracked_by: agent-ix/quoin#204` from
the day it was written. #204 is the mocked-confirmation ticket. Two findings
sharing one ticket means closing it closes both, and only one was ever worked —
so this family had no owner and nothing said so.

`agent-ix/quoin#224` filed. TC-963 now requires every undetectable finding to
name a distinct ticket.

### FND-004 — a boolean over a three-valued fact

`now_detectable: true` was flattening two genuinely different states:

- **AK-001** — `no-symbol-bound` carries a `path`. A reader gets a file.
- **AK-003** — `coverage.specific_shaped` is a ratio over the whole corpus. A
  reader gets a number that moved, and 274 spec files to search.

Counting those as one overstates the toolchain. `detection_strength` is now
`located | aggregate | none`, declared with its scale in the file, and the
re-adjudicated distribution is **2 located, 2 aggregate, 3 none** — against the
old reading of "4 detectable, 1 partial, 2 not".

It is the same conflation `finding_localisation_rate` exists to expose, and the
two now agree: 40% on the first scored tier-1 run, 2 of 5.

## The pin is unchanged

`pinned_sha` stays at `fc5d644`. Re-pinning requires re-adjudicating against a
re-read tree, and this pass re-adjudicated the **detectors**, not the corpus.
The findings are still claims about that specific tree; the corrections are about
whether the toolchain can see them.

Note that `filament-ide-rs` main has since moved to `6f87a7e` (PR #505), which
carries fixes for several of these findings. That does not touch the key: the
pinned tree is immutable, and `make battletest` refuses to score a corpus sitting
off its pin.

## Coverage

Re-adjudication method, per entry: locate the detector in source, locate its
caller, and confirm the behaviour against the tier-1 corpus that seeds the
family. Three entries changed state; four were confirmed unchanged with their
strength recorded for the first time.

| id     | family                    | was            | is                   | checked by                                              |
| ------ | ------------------------- | -------------- | -------------------- | ------------------------------------------------------- |
| AK-001 | `marker-form-mismatch`    | detectable     | **located**          | tier-1 `marker-mismatch`, precision 1.00                |
| AK-002 | `hollow-denominator`      | detectable     | **aggregate**        | tier-1 `hollow-metric`; diagnostic carries `path: null` |
| AK-003 | `catch-all-universal`     | detectable     | **aggregate**        | metric-only by construction                             |
| AK-004 | `vacuous-under-guard`     | detectable     | **located**          | tier-1 corpus, `src/lib.rs:7`                           |
| AK-005 | `oracle-is-code-copy`     | partially      | **none**             | no production caller for `oracle_copies`                |
| AK-006 | `mocked-confirmation`     | not detectable | **none**, new reason | `injections` never supplied                             |
| AK-007 | `gate-that-gates-nothing` | not detectable | **none**, new ticket | no detector exists                                      |

TC-961, TC-962 and TC-963 pin all three corrections, and each fails if the prior
state is restored: a strength disagreeing with `now_detectable`, a `false` entry
carrying a date or a fix, and two findings sharing a ticket.
