---
id: SR-159
title: "Dispositions for the EPIC 373 Rust burn-down spec review set"
type: SpecReview
analysis: base
scope: "SR-151..SR-158; FND-173..FND-186; ADR-0003, StR-009, US-024, FR-096..FR-103, NFR-024..NFR-027, IT-003"
review_set: all
---

# SR-159: Dispositions for the EPIC 373 Rust burn-down spec review set

## Summary

Eight analyses ran over the burn-down spec set and raised fourteen findings,
FND-173 through FND-186 — five high, seven medium, two low. This records what
each one caused.

**Eleven are resolved, three remain open.** Of the five high findings, three are
resolved and two are open; both open highs are the same subject and are deferred
to the stage that owns it rather than fixed here.

The review files themselves are not edited. A review records what was true when
it ran, and rewriting one to say the finding went away destroys the evidence that
it was ever raised. This document is where the outcome lives, which is also the
answer to the problem it exists to solve: a reviewer meeting six high findings on
a branch could not tell which were live, because nothing recorded that four had
already been answered — three of them by a commit on that same branch.

## Findings

FND-173..FND-186 are the eight analyses' findings and their dispositions.
FND-1483 and FND-1484 were raised by the disposition pass itself and are recorded
here rather than appended to `risk-complexity.md`, for the reason stated at the
end of this document: a review records what was true when it ran, and adding
findings to it afterwards makes it a document that cannot be dated. This one can.


| ID | Severity | Summary | Refs |
|---|---|---|---|
| FND-173 | high | **Resolved** — the other way | The finding argued `skills/**/workflow-assets/**` should not be classified as first-party executable logic in scope. The matrix agrees: `skills/**` is **Allowed** under the `agent-skills` exception, so its 23,164 lines were never counted as debt. Recorded in `.language-allowances.yaml` and `docs/rust-burndown/executable-path-matrix.md`. The vendoring defect the finding also describes is [#374](https://github.com/agent-ix/quoin/issues/374), a supply-chain ticket, not a port ticket. |
| FND-174 | high | **Open** — deferred to Stage 3 | `ajv-formats` is not a dependency, so `"format"` keywords are inert annotations under the retained validator, and the set never states whether a Rust validator should assert them. Verified unaddressed: `format` assertion appears nowhere in FR-098 or FR-099. This is the ajv ↔ `jsonschema` error-shape surface and belongs to the stage that owns it ([#378](https://github.com/agent-ix/quoin/issues/378)), not to this branch. |
| FND-175 | high | **Resolved** — `5968de3` | The "exactly one crate" clause omitted sha256 record identity. It no longer does: FR-098 and FR-100 both read "sha256 record-identity computation". Traced — zero occurrences at `30425e6`, one at `5968de3`. FR-098 goes further and names the store: *"the `sha256:<hex>` record identifiers and `sha256-<hex>.json` file names the retained evidence records use"*. `FR-100-AC-8` closes it with a static check over a named non-empty population that fails when a second implementation is planted — the only bypass probe in the set, and the pattern to cite when NFR-025's guard comes up. |
| FND-176 | high | **Resolved** — `5968de3` | The `StrictJsonParser` refusal boundary is now specified: FR-098 requires porting "the retained strict JSON parser's refusal boundary — its refusal of a UTF-8 byte-order mark, of non-fatal UTF-8 and of trailing content". |
| FND-177 | high | **Open** — deferred to Stage 3 | ajv `strict` mode disagrees across five call sites, and `allErrors: true` makes the error *set* part of what FR-098-AC-2 compares. Verified unaddressed: `strict` and `allErrors` appear nowhere in FR-098. Same subject and same owner as FND-174. |
| FND-178 | medium | **Resolved** — `5968de3` | FR-096-AC-6 pinned the payload at exactly the buffer cap. It now reads "under a declared buffer ceiling **strictly greater than that size**". |
| FND-179 | medium | **Resolved** — `5968de3` | The tautology is gone: FR-099-AC-8 now asserts over `package.json` rather than over a Cargo workspace, which could not have expressed an npm dependency either way. |
| FND-180 | medium | **Resolved** — `5968de3` | FR-102 now records the repository's actual published state — `@agent-ix/quoin` declaring `registry.npmjs.org` with public access — rather than a constraint contradicting it. |
| FND-181 | medium | **Resolved** — `5968de3` | NFR-026-AC-4 no longer reaches outside the repository: it compares **a checked-in record** of each upstream's declared channel against this workspace's, and names the disagreement. |
| FND-182 | medium | **Open** | FR-098-AC-3 still replays "every digest in every reachable store" with nothing enumerating that population. It now carries a planted-single-byte probe, which answers half the finding — the check is demonstrably not vacuous — but a probe proves the check fires, not that the population is complete. Enumerate the stores or name the enumeration as the gate's own precondition. |
| FND-183 | medium | **Resolved** — by the artefacts existing | The finding said the metric's inputs did not exist: stage tickets 0–9 were epic prose and `.language-allowances.yaml` was absent. Both exist — [#375](https://github.com/agent-ix/quoin/issues/375) through [#381](https://github.com/agent-ix/quoin/issues/381) plus [#382](https://github.com/agent-ix/quoin/issues/382)–[#389](https://github.com/agent-ix/quoin/issues/389), and the manifest at the repository root — and the matrix links each row to the ticket that retires it. |
| FND-184 | medium | **Resolved** — `5968de3` | FR-102 now names `@agent-ix/filament-plan-sync` as a runtime dependency, and FR-102-AC-3 requires a dated owner disposition covering the `plugins` array and the `command_not_found` hook together. |
| FND-185 | low | **Resolved** — by `#401` landing | The artefacts described in the present tense now exist: `src/core/exec.ts`, `src/core/index.ts` and `src/core/reference.ts` are on `origin/main`. |
| FND-186 | low | **Resolved** — `ed5cceb` | The finding caught a stale test-file count, and chasing it found the matrix was stale in a way the finding did not anticipate — [#388](https://github.com/agent-ix/quoin/issues/388) deleted nine more test files after the matrix was written. Re-measured: 344 files, 102,248 lines. See *Two rules* below. |
| FND-1483 | high | **New, raised by this pass, not by the eight analyses.** An instrument can agree with itself and report that as agreement with the world. Checking an installed module set against a local upstream checkout is ONE direction wearing two coats: both are pulled copies, both age at the same rate, and they agree with each other precisely while both disagree with the remote. I asserted "not a stale pin — I checked both directions" on that basis and was wrong in the opposite direction from the truth; the repository was ahead the whole time. The check is only two-directional when one side is a remote ref fetched in the same breath. Same shape as a replay gate passing over zero comparisons, `status_lies: []` over four unclassified tables, and `NFR-024-AC-8` governing an empty category — four instances in one day, four different subjects. | NFR-024-AC-5; NFR-027; FR-098-AC-3 |
| FND-1484 | medium | **New, raised by this pass.** A whitespace-only change is invisible to diff review and fatal to a digest. Resolving a rebase conflict in the `Makefile` dropped one blank line, which changed its sha256 and therefore its entry in `quality/verification-stack-lock.json`; the file was otherwise byte-identical to `origin/main`. Caught by re-running the digest check, not by reading the diff. It is the counter-example to the #350 phantom digest and belongs beside it: there the digest fired with nothing behind it, here it fired with exactly one character behind it. Both are the same instrument, and a review recording only the false alarm teaches that the instrument is noisy. | NFR-025; NFR-027 |

## A gap neither the review nor the fixes named — recorded, not fixed

`FR-100-AC-8`'s planted-second-implementation probe covers canonical JSON, JCS,
sha256 record identity and blake3. It does **not** cover the sha256
**raw-file-reference** role, which the crate implements as `RawFileSha256` /
`digest_file_sha256` in `rust/quoin-store/src/digest.rs`.

So a second raw-file digest path could be planted and the probe would not fire.
Same shape as FND-175, one role down, much lower stakes — a raw-file reference is
not an identifier inside a retained record.

There are three sha256-or-blake3 roles in the retained tree and they are not
interchangeable:

| role | algorithm and rendering | owner |
|---|---|---|
| assurance-record identity, and its file names | `sha256:<hex>` ids, `sha256-<hex>.json` files | `src/evidence/assurance-records.ts` |
| change-assurance record digests | blake3 over JCS bytes, bare `<hex>.json` files | `src/change-assurance/store.ts` |
| raw-evidence file references | `sha256:<hex>` | `src/measurement/intervention.ts` |
| vendored-schema provenance pinning | sha256, **bare hex** | `src/quire/contract.ts:103` |

**The test for whether two roles are the same domain:** an implementation of one
can be substituted for the other **without a rendering change**. Rendering is the
operative property, not the algorithm — a bare-hex sha256 and a `sha256:`-prefixed
sha256 are not interchangeable at any call site that parses or constructs the
string, which is every call site that matters. `src/evidence/assurance-records.ts:522`
rejecting a bare hex against `/^sha256:[0-9a-f]{64}$/` is that proof in the
retained code.

Under that test none of the four collapse — and the pair that looks collapsible,
sharing both algorithm and prefix, is separated by role rather than by rendering,
which is why it needs the map rather than the test. The fourth role,
`src/quire/contract.ts:103` `schemaHash`, is bare-hex sha256 over vendored schema
bytes for provenance pinning: distinguishable from the raw-file-reference role by
rendering, so its own domain. Owner ruling 2026-09-12.

Not fixed into this branch. It is a decision to take, not a defect to patch, and
the branch is not held for it.

## Which document is state and which is evidence

`base.md` books FND-174, FND-177 and FND-182 as **Resolved**. This document books
the same three as **Open**, and this document is correct.

That is not a contradiction to be repaired by editing `base.md`. A review records
what was true when it ran; this document records what happened afterwards. When
they disagree, **this one is state and the review is evidence**, and a reader who
reaches `base.md` first should come here before acting on it.

The three were re-checked rather than inherited: FND-174's `format` assertion and
FND-177's `allErrors` error-set cardinality are absent from FR-098 today, and
FND-182's population is still unenumerated though it has since gained a probe.
Both are deferred to [#378](https://github.com/agent-ix/quoin/issues/378), which
owns the ajv error-shape surface.

## Two rules this review set produced

**FP-NRX is re-measured at a revision, never adjusted by deltas.** Refreshing the
matrix by subtracting #388's 3,200 lines from 81,910 gives 78,710. The measured
value is **78,669**. The 41-line gap is 185 redundant `// Trace:` lines merged
away during the tag pass, netted against the other movements — nobody could have
derived it from the deltas, and the subtraction produces a number that looks right
and is wrong. Every figure states the revision it was measured at.

**A review file is evidence, not state.** Four findings were answered by a commit
on the same branch that carries the review, and the review still reads them as
open, because a review records what was true when it ran. The disposition belongs
in a document like this one. Editing the review to say the finding went away
destroys the record that it was ever raised — and leaves the next reader with the
same problem, one branch later.
