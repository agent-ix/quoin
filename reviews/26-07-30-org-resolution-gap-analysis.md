---
id: SR-002
title: "Gap-analysis review of authoring-organization resolution (US-010, FR-025)"
type: SpecReview
analysis: gap-analysis
scope: "FR-025; US-010; FR-023-AC-4; FR-014; spec/matrix.md TM-001; src/org.ts; tests/org.test.ts, write.test.ts, cli.test.ts; spec-artifacts-iso + spec-artifacts-process skeletons"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-025"
    type: "reviews"
  - target: "ix://agent-ix/quoin/TM-001"
    type: "references"
---

## Summary

Post-implementation gate over the authoring-organization work on
`feat/org-resolution`: US-010, FR-025 (7 ACs), the FR-023 configuration-surface
amendment, and the accompanying `example-org` change in the two spec-artifacts
modules.

The capability is real and faithfully implemented. `quoin` previously had no org
configuration at all — no flag, no environment variable, no config key, and no
occurrence of `org` in `src/` — so an author outside the publishing organization
could only hand-type frontmatter, guided by two sources that contradicted each
other. `resolveOrg` now answers from `--org`, `QUOIN_ORG`, or the repository's
own remote, and reports an unresolved org rather than inventing one. All seven
ACs carry tracking tags and are backed by tests that exercise real code paths.

The gate nonetheless returns **FAIL**, on one gap that is purely procedural and
one that is substantive. The Test Matrix (TM-001) has no rows for FR-025 or
US-010, so the component's coverage contract does not yet acknowledge the new
requirement — `/spec-matrix` has not been run. Substantively, FR-025's Behavior
section now contradicts its own implementation: it states the command reads
`.git/config` directly, but worktree and submodule resolution (added during code
review, and the case that matters most in this ecosystem) reads the config in the
common directory a `gitdir:` pointer names. Five further behaviors in `src/org.ts`
have no owning clause, and the AC-7 test was shown by experiment not to prove its
requirement.

No finding here disputes the design. The two deliberate divergences from the
sibling implementation in `filament-ide-rs` — failing loudly instead of falling
back to a `local` sentinel, and refusing to derive an org from a remote that
names none — are both consistent with FR-025's stated rationale and are the
correct calls for human-facing declared identity.

## Verdict

**FAIL** — TM-001 carries no Test Case rows for FR-025 or US-010 (Step 2), and
FND-002 is a `high`-severity spec-faithfulness gap. Both are mechanical to
close; neither indicates the implementation is wrong.

There is no plan bundle for this work (`plan/` does not exist in this
repository), so Step 1 (plan completion) is not applicable rather than failed.
The work went spec → code directly without `/spec-to-plan`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TM-001 has no rows for FR-025 or US-010. Every other FR carries a matrix row naming concrete tests as `file :: "test name"`; the seven FR-025 ACs and US-010 are absent, so the coverage contract does not record the new requirement. FR-024 is likewise absent, predating this change. Closing it needs `/spec-matrix` to add rows for FR-025 (org.test.ts, write.test.ts, cli.test.ts), US-010, and FR-024. | ix://agent-ix/quoin/TM-001; ix://agent-ix/quoin/FR-025; ix://agent-ix/quoin/US-010 |
| FND-002 | high | FR-025's Behavior states the command "SHALL read `.git/config` directly", which is now false. In a worktree or submodule `.git` is a file holding a `gitdir:` pointer, and `resolveGitDir` follows it and the `commondir` it names to read the config in the shared common directory. The implementation is correct and desirable; the requirement text is stale and must be amended to describe both layouts. | ix://agent-ix/quoin/FR-025 |
| FND-003 | medium | The AC-7 test does not prove AC-7. It sets `process.env.PATH = ""` and asserts resolution still works, but `resolveOrg` spawns no subprocess, so the test only shows no PATH lookup was required. Verified by experiment: `execFileSync("/usr/bin/git", …)` succeeds with `PATH=""`, so an implementation that shelled out via an absolute path would pass unchanged. A faithful test spies on `node:child_process` and asserts no spawn occurs. | ix://agent-ix/quoin/FR-025 |
| FND-004 | medium | Five behaviors in `src/org.ts` have no owning clause in FR-025: rejecting local-path remotes (`/srv/git/repo.git`, `../sibling`, `file://`) as naming no organization; qualifying a nested namespace by its innermost group (`org/subgroup/repo` yields `subgroup`); case-insensitive `[REMOTE]` section matching with a case-sensitive subsection; worktree/submodule resolution; and trimming blank `--org`/`QUOIN_ORG` values. The local-path rule and the namespace rule are semantic decisions that change the emitted org and warrant explicit clauses. | ix://agent-ix/quoin/FR-025 |
| FND-005 | low | The worktree describe block in `tests/org.test.ts` is tagged `FR-025-AC-2`, but AC-2 reads "The organization is parsed from an SSH remote URL". The tests exercise `.git`-file and `commondir` resolution, not SSH URL parsing. The tag should move to whichever clause FND-002 and FND-004 add for worktree layouts. | ix://agent-ix/quoin/FR-025 |
| FND-006 | low | The AC-5 assertion `expect(UNRESOLVED_ORG_MESSAGE).toContain("--org")` tests a module constant rather than behavior. It is redundant given the write.test.ts assertion on the rendered pack, which does verify the remedy reaches the author. Harmless, but it inflates apparent AC-5 coverage. | ix://agent-ix/quoin/FR-025 |
| FND-007 | low | Neither spec-artifacts module needs a requirement change for the `example-org` edit — no requirement names a specific organization, so the change is editorial. It is governed by, and conforms to, spec-artifacts-iso FR-002, which requires each archetype to ship "the canonical example an author fills", explicitly replacing "placeholder defaults". That clause independently vindicates the concrete `example-org` value over a `<your-org>` placeholder in skeleton frontmatter. | ix://agent-ix/spec-artifacts-iso/FR-002 |
| FND-008 | low | The two spec-artifacts changes are unverified by their own test suites: no poetry or python toolchain was available, so `tests/test_manifest_and_validate.py` — which asserts each filled skeleton passes `validate_document` — did not run. `quire validate` over the skeletons exits 0, unchanged from the pre-edit baseline, and neither suite hardcodes the old organization, so the risk is low but not zero. | ix://agent-ix/spec-artifacts-iso/FR-002 |

## Coverage

**Step 1 — Plan completion: N/A.** No `plan/` bundle exists in this repository;
the work proceeded spec → code without `/spec-to-plan`. Nothing to assert.

**Step 2 — Matrix verification: FAIL.** All seven FR-025 ACs carry tracking tags
in test code (`FR-025-AC-1` through `AC-7`), and FR-023-AC-4 is tagged in
`cli.test.ts`. Every tag resolves to a test that exercises real code — none are
stubs, skips, or assertion-free. The failure is in the other direction: TM-001
records no Test Case rows for FR-025 or US-010, so the matrix does not yet claim
the coverage the tests provide (FND-001).

Verification-column accuracy was checked and is now sound. FR-025-AC-6 and
FR-023-AC-4 both name `cli.test.ts`, which contained no org tests when the
requirements were written; three were added during code review, so the columns
are no longer aspirational.

**Step 3 — Underspecified code: partial.** `resolveOrg`, `originOrg`, the pack
fields, and the `--org` flag all trace to FR-025 or FR-023. Five behaviors do not
(FND-004), and one contradicts its requirement (FND-002). No stubs, TODOs, or
placeholder returns were found in the new code.

**Step 4 — Semantic review: run, scoped to FR-025.** Per-AC judgment of
intent↔test↔code agreement:

- **AC-1 (precedence)** — strong. Three distinct decoy values (`from-flag`,
  `from-env`, `from-git`) make each assertion discriminating, and exact object
  equality pins `source` as well as `org`.
- **AC-2 / AC-3 (SSH, HTTPS)** — hold, and are stronger than the ACs require:
  ports, `ssh://`, missing `.git` suffix, trailing slashes, and multi-remote
  configs are all covered. Mis-tagging noted in FND-005.
- **AC-4 (unresolved without failing)** — all three named sub-cases present; the
  "without failing" clause is proven implicitly by asserting a returned value.
- **AC-5 (remedy, no substituted value)** — the `not.toContain("agent-ix")`
  assertion is a genuine anti-regression guard on the original defect. One
  redundant assertion noted in FND-006.
- **AC-6 (pack carries org and source)** — covered in text, in the object, and
  through the real CLI `--json` path.
- **AC-7 (no git on PATH)** — test does not prove the requirement (FND-003).

Code matches requirement intent throughout, including where it exceeds the
written clauses. The wrong-org cases found during code review — a hostname, a
path segment, and `..` each returned as a confident organization — were repaired
before this gate and are now covered by regression tests.

**Suite state.** 134 passed, 3 failed. The three failures reproduce identically
on a clean `origin/main` checkout (`skeletonPath` returns `FR.md` where the tests
expect `fr.md`, a macOS case-insensitive filesystem issue) and are unrelated to
this work. They do mean the local gate is red on `main` independently of this
branch.
