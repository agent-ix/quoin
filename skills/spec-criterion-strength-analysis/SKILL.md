---
name: spec-criterion-strength-analysis
description: Judge whether each acceptance criterion can actually fail, using Jev (typesafe.ai System One) for calibrated semantic judgment over spec text only. Emits a validated SpecReview recording weakness_kind per AC and adverse_case_coverage per FR.
---

# Criterion-Strength Analysis

Specified by PLAT-837. Use this skill to judge whether an acceptance criterion
**can actually fail**, as distinct from whether it is backed by a passing test.
A spec can reach 100% matrix coverage and mean nothing, because the ACs were
written to be satisfiable by whatever got built — `spec-review`'s Common Issues
already names "Vague Criteria" as something to notice by eye, but nothing in
the pipeline measures it. This lens does.

**This lens reads spec text only.** No code, no trace tags, no AST, no test
bodies. It has no dependency on the AST symbol-table work landing in
`quire-rs/src/symbols/`, and it never reads a test to decide whether the *AC*
it names is well-formed — that is `gap-analysis`'s `assertion_vacuous`
question (PLAT-839), a different lens judging a different pairing (test vs.
requirement, not AC vs. itself).

## Status: blocked on two dependencies not owned by this skill

**Do not run this skill yet.** Two things it depends on are being built by
other agents in parallel, and this skill must not paper over either:

1. **The `analysis: criterion-strength` schema value**, owned by another
   agent, not this skill. Per `spec-review`'s own rule: *"a selected analysis
   absent from the installed `SpecReview.analysis` schema is an unavailable
   dependency, not permission to emit an invalid review document."* **Check
   the installed `SpecReview.analysis` enum at run time, every run** — do not
   trust a point-in-time record of whether `criterion-strength` has landed,
   since that fact changes the moment the owning change merges and a stale
   note would silently stop being true. If `criterion-strength` is absent,
   stop and say so rather than writing a `spec/reviews/criterion-strength.md`
   that fails `quire validate`.
2. **The Jev client.** This skill has no HTTP client and adds no third-party
   dependency — that is a separate, later task, gated on its own approval. Step
   2 below cannot execute until that task lands.

What exists today, ready for that later task to consume unchanged:

- [`assets/question-set.json`](assets/question-set.json) — the five `noul`
  questions, the `weakness_kind` choice enum (six members), and the
  `adverse_case_coverage` score rubric, transcribed verbatim from PLAT-837
  rather than paraphrased, as a serialisable data artifact a Jev client can
  hand to the API unchanged. Per the request shape PLAT-837 states: one
  request per FR, one question block per AC row, each carrying the FR
  statement, the AC row, and the FR's Description/Behaviour/Constraints.
- [`assets/fixtures/criterion-strength-fixtures.json`](assets/fixtures/criterion-strength-fixtures.json)
  — the labelled adverse-case corpus. See **Fixtures** below.

## Findings only

Same authority rule as every other lens in this family, and stated in
PLAT-837 itself: **this lens never edits an AC, never authors a requirement,
never changes a matrix row.** Its only write is the `SpecReview` artifact.

A low-confidence verdict is **annotated, not suppressed** — the finding still
appears in the table marked unconfirmed. This is the opposite of the
`ix-board` low-confidence rule (which suppresses an edge write); the two are
opposite because one mutates and one reports, per PLAT-837's own note. Do not
copy the `ix-board` rule across by reflex.

## Process (once both dependencies land)

1. **Resolve the repo and spec glob**, same as `spec-ears-analysis` and
   `spec-correctness`: default `spec/**/*.md`.
2. **Check the schema.** Confirm `criterion-strength` is in the installed
   `SpecReview.analysis` enum. If not, stop; report the blocker; do not emit.
3. **Build one request per FR.** For each FR: the FR statement, its
   Description/Behaviour/Constraints, and one question block per AC row, per
   `assets/question-set.json`.
4. **Call Jev** for each AC row's five `noul` questions, the `weakness_kind`
   `choice`, and — once per FR, over the full AC set — the
   `adverse_case_coverage` `score`. This step needs the client the later task
   adds; nothing here performs it.
5. **Author the `SpecReview`.** Fetch the template once with
   `quoin write --types SpecReview`. Write one document to
   `spec/reviews/criterion-strength.md` with `analysis: criterion-strength` in
   the frontmatter, a `## Summary`, and a validated `## Findings` table
   (`| ID | Severity | Summary | Refs |`, `FND-NNN` ids, Severity ∈
   `low`/`medium`/`high`). A below-threshold `weakness_kind` or
   `adverse_case_coverage` verdict is still a row in this table, marked
   unconfirmed — never dropped.
6. **Validate.** `quire validate --scope <repo> "spec/**/*.md"`.

> **`--scope` is the repository root, and must be passed explicitly.** Since
> quire-cli v0.16.0 (quire-rs CR-045) the command derives two roots from it and
> never interchanges them: spec documents come from `<repo>/spec` only, trace
> tags from the source tree at `<repo>` excluding `spec/`. Check
> `quire --version` >= 0.16.0 before relying on this.

## Severity mapping

PLAT-837 states the enum and the rubric but not a severity table; this skill
adopts the mapping every sibling analysis skill in this repository uses —
severity reflects how much the weakness degrades what the AC can prove, not
how it happens to sort in the enum:

- `unfalsifiable` -> **high** (the AC cannot fail; the matrix row it backs
  proves nothing).
- `implementation_coupled` -> **medium** (the AC over-specifies; it will pass
  forever once written and survive a refactor that should have broken it).
- `unmeasurable_threshold` -> **medium** (asserts a quantity it never states).
- `restates_requirement` -> **low** (adds no discriminating power beyond the
  FR sentence, but is not itself false).
- `happy_path_only` -> **low**, as its own per-AC finding.
- `sound` -> no finding.

`happy_path_only` is **both** a per-AC `weakness_kind` finding at `low` and an
input to the FR-level `adverse_case_coverage` score — the two are not
alternatives (an earlier draft of this skill treated them as one-or-the-other
and disagreed with the fixture corpus, which already carried a `low`-shaped
per-AC `happy_path_only` verdict; fixed to match). A row labelled
`happy_path_only` files a finding at the row, the same as any other
`weakness_kind`, and it also contributes to whatever `adverse_case_coverage`
score the skill computes once per FR over the whole AC set — see
`CS-FIX-011`/`CS-FIX-014` in the fixture corpus for a worked example of the
same underlying gap observed at both granularities, and the `entangled_with`
note on each explaining why that is one piece of evidence, not two.

A low-confidence verdict in any category is reported at its mapped severity
and marked unconfirmed, never silently dropped to `low`.

## Fixtures: the adverse-case corpus

`assets/fixtures/criterion-strength-fixtures.json` holds real acceptance
criteria drawn from specs already in this org's repositories (`quoin`,
`quire-rs`, `spec-hierarchy`), each with a human label and a rationale citing
the criterion's own text — never constructed examples. It carries:

- 11 `weakness_kind` fixtures, covering 5 of the enum's 6 members with a
  primary label. `unmeasurable_threshold` has **no** clean exemplar in this
  corpus — searched across 20 repos in this org and none was found (see the
  file's top-level `unmeasurable_threshold_exemplar_search` note); declared as
  a gap rather than filled with a mislabelled placeholder.
- 4 `adverse_case_coverage` fixtures (FR-level): a clean 0, a clean 2, a
  contested 1 (alternate 2), and a contested 2 (alternate 3) — covering the
  rubric's full 0–3 range, including both of its disputed boundaries with an
  unambiguous level-2 exemplar sitting between them.
- 9 of the 15 fixtures marked `confidence: "ambiguous"` — cases the lens is
  expected to find hard, not clean positives and negatives.

**Disagreements are recorded, not resolved.** A second, independent reader
re-derived every label in this corpus and disputed 5 of the original 14 (see
the file's top-level `governing_ruling_on_disagreement` field). Every fixture
where the two readings differ carries a `weakness_kind_contested` or
`adverse_case_coverage_contested` array naming both, plus a `confidence_note`
identifying the disagreement's source — never a silent pick of one reading
over the other. An answer key with a written-down disagreement is worth more
than one reader's confident labels, because the corpus exists to grade a
classifier, and grading against a label that was contestable but nobody
contested is the failure mode this convention exists to catch.

This corpus is **not** the TC-145 `ParsedFile`/`Sync` fixture recorded in
PLAT-839's comments. That fixture is a test-vs-requirement divergence case for
PLAT-839's `gap-analysis` semantic lens; this lens judges an AC's own text in
isolation, never against test code, so it is out of scope here by
construction — not merely left out.

This corpus is data for **M2** (accuracy against ground truth) in PLAT-837's
MeasurementPlan once the Jev client exists. It has not been run through any
classifier — see **What is not done yet** below.

## What is not done yet — PLAT-837 acceptance criteria still unmet

Stated plainly, so nothing here is read as a working lens:

- **The Jev API call itself.** No client exists in this repository or this
  change; adding one is a separate, later, approval-gated task. Nothing in
  this skill can execute end to end without it.
- **`spec/reviews/criterion-strength.md` does not exist.** PLAT-837's
  acceptance requires it to validate under `quire validate --scope <repo>`
  with the new `analysis:` value. It cannot be authored — even as a stub —
  without a real run to summarise, and the schema value it would declare is
  not confirmed landed (see **Status** above). Emitting one now, before either
  dependency lands, is exactly the failure mode `spec-review` warns against:
  an invalid document standing in for permission.
- **Every classifier-derived engineering-assurance metric in PLAT-837's
  MeasurementPlan section (M1 disagreement rate, M2 accuracy/precision/recall,
  M3 calibration error, M4 abstention rate and precision, M5 cost/latency, M6
  spec defect escape rate, M7 adverse-case coverage distribution).** All seven
  require running the classifier this skill cannot yet call. The fixture
  corpus above is the M2 ground truth these metrics will be measured against,
  not a substitute for having measured them.
- **The M6 feedback path** (recording which lens verdict preceded a
  late-discovered weak AC) does not exist yet; PLAT-837 puts building it in
  scope for the ticket, and it has no owner in this change.
- **A formal `MeasurementPlan` document** (`spec/assurance/MP-XXX-*.md`) for
  this lens, which PLAT-837's "Engineering assurance" section asks to be
  authored *before* implementation. Not written here: the `spec/assurance`
  MeasurementPlan family carries its own reporting-semantics contract
  (`corpus/spec/evidence/measurements`, the reporting-case corpus under
  `corpus/cases/reporting/`) that this change does not touch and should not
  guess at. Authoring one correctly is left to whoever picks up the Jev
  integration, when there is a real run to define a baseline against.
- **`Severity` above is this skill's own default mapping**, not one PLAT-837
  states; it should be revisited once M2/M3 data exists to check it against.

## When to use

Not yet — see **Status**. Once both dependencies land: on an FR whose ACs are
about to be trusted as a merge gate, the same way `spec-ears-analysis` and
`spec-correctness` are used before or alongside `gap-analysis`.

Not for: authoring or rewording criteria (`specify`), building the matrix
(`spec-matrix`), or judging whether a test that backs a criterion is vacuous
(`gap-analysis`'s `assertion_vacuous`, PLAT-839 — a different pairing, a
different lens).
