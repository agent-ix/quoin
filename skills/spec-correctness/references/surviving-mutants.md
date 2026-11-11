# Surviving mutants (optional input to steps 2 and 4)

**Goal**: aim a regenerated test at behaviour a criterion's existing tests provably do not
detect, using mutation evidence somebody else already recorded.

Quoin never runs a mutation tool. The consumer's CI does, and `quoin evidence record`
transcribes the run. When a mutation floor is declared and missed, the audit reports each
recorded survivor.

## Read

```
quoin evidence audit --json --mutation-floor <criticality>=<ratio>
```

Keep the findings whose `kind` is `surviving-mutant`. Group them by `obligation`: that value
is the criterion id, equal to the classification record's `row_id`. Each finding carries:

- `symbol` — `path::function`, the production function that was mutated
- `path` and `line` — where, when the tool reported a line
- `summary` — the mutation that went undetected

No such findings, or no floor declared: skip this reference. Absence says nothing about the
tests, so emit nothing about it.

## Use

- **Step 2.** For a record whose `row_id` has survivors, ground its oracle on the survivor's
  `symbol` first, and cite `path:line` as evidence.
- **Step 4.** For each survivor, emit one property whose oracle fails under that mutation
  (the mutation names the behaviour the old tests ignored). Tag it with the same `row_id`.
- **Step 6.** A survivor you could not ground goes in the review artifact as a finding with
  the survivor's `symbol` and `line`. The criterion may simply never have demanded that
  behaviour; say so rather than inventing an oracle.

Survivor lists are bounded per obligation. A short list is not the complete set.
