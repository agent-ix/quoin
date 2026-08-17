---
name: spec-evidence-analysis
description: Recommend verification methods per obligation from the declared catalog, and flag mismatches with what was authored.
---

# Verification & Evidence Strategy

Every requirement needs a defined verification method and a concrete artifact
that proves it was verified. This skill recommends the method **from the
declared catalog**, and flags where the authored choice disagrees.

> **This skill no longer carries its own method table.** It used to: a
> skill-local list of `test | analysis | inspection | demonstration` plus a few
> evidence kinds, declared in no manifest and read by no code. The result was
> `Verification` columns defaulting to `Test` by habit, with nothing ever
> advising DAST for an attack surface, monitors for a temporal property, or
> fault injection for a reliability NFR. The methods now live in module data
> (quire-rs FR-054, `spec-artifacts-process` FR-007) and are read from there.

## When to use

- At **Spec Review**, to confirm each obligation's method before the matrix is built.
- At **Matrix** time, to plan which suites the chosen methods imply.
- Whenever a new NFR is added — NFRs are the ones most prone to a defaulted method.

## Run the advisor, do not recall the table

```bash
quoin advise                     # every obligation, with its recommendations
quoin advise --mismatch-only     # only where the authored method disagrees
quoin advise --inconclusive-only # only where no rule matched — your work list
quoin advise --json              # for scripting
```

`quoin advise` derives the obligations from `quire coverage --json`, reads each
criterion's FR-052 property shape from `quire properties --json`, and matches
both against the catalog's `applicability` rules. **This is the deterministic
half of the analysis and it is not optional** — a method you recalled is a
method nobody can check.

The catalog behind it:

```bash
quoin catalog methods            # human-readable, grouped by class
quoin catalog methods --json     # the merged catalog, for scripting
quoin catalog methods --class Analysis
```

Merged first-wins across active modules, exactly as quire-rs merges it — so the
method you are advised is the method the auditor will later check conformance
against.

## Process

1. **Run `quoin advise`.** It carries the obligations (quire-rs FR-053: id,
   statement, hash, authored method, criticality) *and* the recommendations.
   Do not restate either by hand.

2. **Take the deterministic recommendations first.** They come from the
   catalog's `applicability` rules matched against facts about the obligation:
   its statement's characteristics, its FR-052 property shape, its archetype.
   Rules match or they do not. Each recommendation names the rule and the value
   that matched, so you can check the reasoning rather than trust it.

3. **Judge only the residue, and label it.** Where the rules are inconclusive
   the advisor says so and stops rather than guessing. That is where your
   judgement belongs — and it must be recorded *as* judgement. Never present an
   LLM conclusion as a verdict (the ADR-0010 discipline).

4. **Confirm or correct the authored method.** A mismatch (`⚠` in the human
   output, `mismatch: true` in JSON) is advisory: the human decides. The confirmed method lands in the spec, becomes the
   obligation's method, and is what `quoin evidence` records discharge against.

5. **Plan the suite.** A method implies an evidence kind; a suite in
   `spec/evidence/suites.md` produces that kind. A recommended method with no
   suite that can produce its evidence is a gap in the plan, not a gap in the
   spec.

## What "inconclusive" means, and why it is not a failure

An obligation the rules cannot place is a normal outcome — the catalog's
applicability rules are deliberately narrow, because a rule that matches
everything advises nothing. Report it as inconclusive and choose a method
yourself. **Do not default to `Test` to make the report look complete**: that is
precisely the habit this skill was rebuilt to end.

## Deliverable

A `SpecReview` with `analysis: evidence`, under `reviews/YY-MM-DD-<slug>.md`.

The `Findings` table carries one row per obligation whose method needs
attention — a mismatch, or an inconclusive recommendation the author must
settle. An obligation whose authored method matches the recommendation needs no
row; a review listing every passing obligation is a review nobody reads to the
end.

```markdown
| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | NFR-009-AC-1 is authored `Inspection`; the statement's `reliability` characteristic recommends `fault-injection` (Test). | NFR-009-AC-1 |
| FND-002 | low | FR-001-AC-9 matched no applicability rule; method chosen by judgement, not by the catalog. | FR-001-AC-9 |
```

Record the per-requirement outcome in the requirement's own `Verification`
cell — that cell **is** the obligation's method (quire-rs FR-053), so editing it
is what changes the plan. There is no separate evidence file to keep in sync.
