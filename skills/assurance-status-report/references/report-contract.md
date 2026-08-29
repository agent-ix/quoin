# Assurance status report contract

Use this contract for a detailed report. Short status requests may compress
sections, but they retain the same evidence and comparison rules.

## Evidence states

- **Proven:** Direct current or retained evidence satisfies the requirement.
- **Contradicted:** Direct evidence shows the requirement is not satisfied.
- **Incomplete:** Some required evidence exists, but the full requirement is not
  demonstrated.
- **Missing:** No authoritative evidence was found.
- **Incomparable:** Values exist, but their definitions or populations do not
  permit a numeric delta.

Never turn incomplete, missing, or incomparable evidence into zero, green, or
complete.

## Required order

### 1. Executive status

Name the goal, current disposition, as-of time and timezone, current promoted
revision, scope boundary, and progress method. Distinguish completion of the
named phase from progress of the larger program.

### 2. Definitions

Define each project-specific term before the metrics table. Definitions cite the
active versioned contract. Prefer plain language, then state the exact counting
rule where it matters.

Examples of distinctions that must remain visible:

- localization identifies the correct place; it does not prove the explanation
  or remedy is correct;
- actionability-field presence does not prove guidance correctness;
- safe refusal preserves uncertainty and does not count as an exact span;
- a controlled-corpus result does not estimate ecosystem-wide precision; and
- a closed ticket does not by itself prove its acceptance criteria.

### 3. Metric progression

Use a table with these conceptual columns:

| Metric | Baseline | Intermediate | Current | Delta | Comparability | Evidence |
| --- | --- | --- | --- | --- | --- | --- |

Every ratio displays `matched/examined` beside the percentage. Each row retains
the metric definition version and population identity, either inline or in its
evidence link. Use `—` for a deliberately absent intermediate snapshot and
`not_computed: REASON` for an unavailable value.

Do not calculate a delta when:

- the definition version changed;
- the applicable population was redefined;
- the source or corpus identity cannot be proved;
- either value is not computed; or
- a producer or required capability changed without an accepted comparison.

When the population merely grows under the same contract, report both the count
change and the rate change. A rate decline with a larger numerator can be scope
growth rather than a product regression; explain which occurred.

### 4. Completed work

Enumerate every in-scope ticket/task with title, repository, final state, delivery
PR or commit, and the evidence that proves acceptance. Group by workstream when
the list is long. Separately enumerate promotion PRs and exact merge commits.

### 5. Open work

Enumerate unresolved items and assign exactly one disposition:

- **blocking:** prevents the named goal;
- **non-blocking:** retained limitation that does not prevent this goal;
- **downstream:** belongs to a later goal or phase;
- **excluded:** explicitly outside the user's scope; or
- **unknown:** relationship or impact is not proven.

Do not omit open work merely because the named phase is complete.

### 6. Drift controls

Report the guard and its proof for each relevant drift class: repository,
submodule, tool/engine, executable bytes, package resolution, toolchain, workflow
action, container, schema, configuration, corpus, metric definition, evidence
overlay, and promotion topology. Distinguish a guard observed in code from a
mutation test proving it fails before governed execution.

### 7. Limitations and judgment

State remaining evidence limitations, then answer whether the effort is moving
toward the goal and why. Use one of:

- **complete** — every requirement is proven and no in-scope work remains;
- **on track** — incomplete, with no current blocking contradiction;
- **at risk** — progress exists, but a named unresolved risk threatens the goal;
- **blocked** — a named blocker prevents meaningful progress; or
- **not started** — no implementation evidence exists.

Close with the next decision or evidence-gathering point. Do not suggest starting
explicitly excluded work.

## Read-only rule

Producing the report does not authorize ticket comments, issue state changes,
branch updates, evidence regeneration, baseline updates, or new external
messages. Ask separately before any such mutation.
