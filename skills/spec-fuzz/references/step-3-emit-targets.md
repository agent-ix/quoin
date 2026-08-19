# Step 3: Emit the targets

**Goal**: a target that compiles, runs, and reconciles.

Templates: [cargo-fuzz](../assets/templates/cargo-fuzz.rs.md) ·
[atheris](../assets/templates/atheris.py.md) ·
[fast-check](../assets/templates/fast-check.ts.md)

## The tag contract

Inherited from `spec-correctness` step 4, unchanged, because `gap-analysis` reconciles by the
same grep. **Two independent carriers**, so one formatting change cannot break reconciliation:

1. `Trace: NFR-003-M-1` on its own line in the doc comment or docstring.
2. The target's own name containing the id — `fuzz_nfr_003_m_1_parse_manifest`.

### Placement, which is where this bites

A carrier binds to the **symbol whose source span encloses it**. For a libfuzzer target that
span is the `fuzz_target!` block — so the doc comment goes immediately above `fuzz_target!`,
not at the top of the file.

A file-header comment reads correctly, passes review, matches a grep, and **binds to
nothing**. Six generated files shipped that way in `@agent-ix/quoin@0.12.x` and every
criterion in them scored zero (`agent-ix/quoin#61`). Do not verify placement with grep — grep
does not care where a comment sits, which is exactly why that shipped.

## The provenance line

One per target, ignored by the reconciliation grep, read by this skill on re-runs:

```
spec-fuzz: obligation=<id> harness=<harness> entry=<symbol> origin=<advised|authored>
```

Add `wrapped=<outer-symbol>` when step 2 grounded on an inner function.

## Idempotent re-runs

Same obligation, same harness, same entry → **leave the file alone** (FR-038-AC-6).

A changed entry point is a real change: rewrite the target and say so in the report. A
changed requirement *statement* is not — the target calls a symbol, not a sentence, so a
reworded requirement with the same surface needs no new file.

## What the target must not do

- **No assertions about correctness.** Fuzzing looks for crashes, hangs and assertion
  failures inside the code under test — not for wrong answers. An `assert_eq!` on the result
  turns every interesting input into a false positive.
- **No `unwrap()` on the result.** The function returning `Err` on garbage is the requirement
  being *met*. Unwrapping turns success into a crash and the fuzzer will find it in seconds.
- **No file, network or process access.** Sanitizers will kill it and the run produces noise.
- **No `panic!` as a way of reporting.** The fuzzer's crash is the report.

The body is therefore usually three lines: take the bytes, make the input the function
expects, call it and drop the result.

## Corpus and seeds

Out of scope. A seed corpus makes a fuzzer dramatically more effective and building one from
the repository's own fixtures is real work with its own decisions.

Say so in the report rather than shipping an empty `corpus/` directory that looks like it was
considered.
