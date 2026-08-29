# Metric-validity study: does "backed" predict test quality?

**Status: designed, not yet run.** `agent-ix/quoin#205`.

## The question

`quire coverage` reports a row as **backed** when a source symbol carries its
trace id. That is a statement about _tagging_, and the whole programme rests on
reading it as a statement about _verification_.

Those are not the same claim, and nothing has ever checked that the first
predicts the second.

If backed rows do not kill mutants at a materially higher rate than unbacked
ones, **the coverage metric needs redefinition rather than re-plumbing** — and
every number this programme has just made honest would be honestly reporting
the wrong thing.

## The design

An independent oracle: `cargo-mutants` kill rates over the pinned tier-2
corpus.

1. Run `cargo mutants` over `agent-ix/filament-ide-rs` at the pinned SHA,
   recording each mutant's file, span and outcome (killed / survived /
   unviable).
2. Run `quire coverage --json` at the same SHA and take `unbacked_rows` plus
   the `verifies` relations behind the backed set.
3. For each mutant, attribute it to the requirement(s) whose `implements`
   markers name the file it mutated (quire-rs FR-062).
4. Partition mutants into those attributable to **backed** rows and those
   attributable to **unbacked** rows.
5. Compare kill rates between the two partitions.

**The prediction, stated before the run**: backed rows kill materially more.
Writing it down first is what makes the result falsifiable rather than
narratable.

## What would invalidate the metric

| result                         | reading                                                                        |
| ------------------------------ | ------------------------------------------------------------------------------ |
| backed kills materially higher | the metric predicts quality; it can be trusted as used                         |
| rates indistinguishable        | **`backed` measures tagging, not verification.** Redefinition, not re-plumbing |
| unbacked kills higher          | the metric is anti-correlated — worse than useless, because it is acted on     |

## Why it is not run here

Three prerequisites, each real:

1. **The corpus must be at the pin.** It is currently at `16eca41`, not the
   adjudicated `fc5d644`, and both `make bench` and `make battletest` refuse to
   score it — correctly, because a score over a different tree answers a
   different question.
2. **`implements` coverage.** Step 3 needs requirement→file attribution, and
   quire-rs CR-082 measured that at 55 of 58 requirements _in quire-rs itself_.
   The tier-2 corpus has not been annotated, so most mutants would attribute to
   nothing and the partition would be mostly empty.
3. **A full `cargo mutants` run over 24 crates is hours**, which is why it is a
   `workflow_dispatch` study and not a gate.

Recording the design without the run is the honest position: the study is
specified, its prediction is committed, and the reasons it has not executed are
stated rather than left as an absence somebody rediscovers.

## Elevation

This elevates `agent-ix/quoin#48` (mutation scoring) from backlog into the
benchmark programme: `cargo-mutants` stops being a tool somebody might run and
becomes the **independent oracle the coverage metric is validated against**.

## Advisory families are not defect-rate estimates

`catch-all-universal` is retained as a corpus-level advisory. It answers
whether a document relies entirely on a catch-all property; it does not claim
that such reliance is wrong. Treating every legitimate firing as a false
positive made precision measure fixture prevalence instead of detector
correctness, so the governed score uses seeded recall and localisation while
reporting precision only over adjudicated rulings. Unadjudicated advisory
firings remain a separate, ratcheted count and cannot disappear into either the
numerator or denominator.

The current Tier-1 collection finds all seeded `catch-all-universal` cases with
the expected loci. Its adjudicated precision basis is 4 true positives, 0 false
positives, and 104 explicitly unadjudicated firings; this is reported as 1.0
over adjudicated rulings, not as a claim that all 108 firings are defects. That
decision and the same treatment for `archetype-matches-nothing` are declared in
`tier1-mapping.json`; changing either family back to defect-shaped scoring
requires a new definition version and comparative evidence.
