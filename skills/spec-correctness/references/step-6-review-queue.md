# Step 6: The Review Queue

**Goal**: everything not settled unattended lands somewhere a person can accept or reject
it, and lands **inert** so it cannot be mistaken for coverage.

## Where it lives

`tests/props/QUEUE.md`, in the test tree. Template:
[`../assets/property-queue-template.md`](../assets/property-queue-template.md).

It is **not** a `SpecReview`. `SpecReview.analysis` is a closed enum in
`spec-artifacts-process` with no `spec-correctness` value, so a file under `reviews/` would
fail `quire validate`. Keeping the queue in the test tree also keeps it out of every
`spec/**` validation glob. Adding the enum value upstream is a reasonable follow-up; it is
not a prerequisite for this skill.

## What goes in it

| Lane | Reason it is queued |
| --- | --- |
| `extraction: candidate` | Metamorphic label the structural pass did not corroborate |
| Second-pass reclassify | An LLM read, not a deterministic one |
| Second-pass witness | An example test, not property coverage |
| `singleton-domain` witness | Classified `extractable`, but grounding found a one-element domain — a `Unit` test, not property coverage |
| Grounded but harness-downgraded | e.g. Python `concurrency` |
| Refused | No test written; the reason is the row |
| Generator library missing | **No file written** — the proposal is a code block in the queue. An import of a missing library breaks collection even for a skipped test |

## Inert by construction

Every queued test carries its harness's skip marker (see step 4). Two consequences worth
being explicit about:

- **Its matrix row is `🚧`, never `✅`.** `gap-analysis` treats a `✅` row backed by a
  skipped test as a *status lie* and raises a `high` finding — correctly. Queued rows are
  in-progress rows.
- **A skipped test still carries its tag**, so `gap-analysis` sees the trace and reports it
  as present-but-not-passing rather than as unbacked.

## Acceptance procedure

For each accepted row:

1. Delete the skip marker line.
2. Move the file out of the review location: `tests/props/_review/` → `tests/props/`
   for TypeScript and Python, `tests/props_review_fr_NNN.rs` → `tests/props_fr_NNN.rs`
   for Rust, whose paths are flat (step 4).
3. Change the provenance line's `review=required` to `review=accepted`, leaving `row=` and
   `origin=` untouched.
4. Run the suite.
5. Flip the matrix row from `🚧` to `✅`.

The tracking tag is **byte-identical** through all of this — that is the point. Nothing
about reconciliation changes when a test is accepted.

Rejection is simpler: delete the file, and leave the queue row with `status: rejected` and
a one-line reason so the next run does not re-propose it. A re-run reads existing
`rejected` rows and skips those `row_id`s unless the criterion's `statement` has changed.

## What the queue must not contain

- A verdict, a score, or a completion percentage framed as a target.
- A recommendation to edit any spec file.
- A `row_id` absent from the classification output.
