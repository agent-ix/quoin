# Step 5: The LLM Second Pass

**Goal**: recover tests from the criteria the deterministic pass could not settle —
`property: example`, `property: unclassified`, and anything step 2 refused to ground.

**Ask before running this** when the residue is large (more than ~30 records). It is an
expensive judgement pass. Fan out one subagent per FR when you do run it.

## Why this pass exists here and not in the engine

quire-rs#45 is closed as answered: **31.3% recall at 93.3% precision is the engine's
deterministic ceiling.** Three widenings were measured factorially against a precision gate
fixed in advance at 85%; one shipped at 100%, two were deleted at 70% and 56.7%. Another
regex cannot get further without surrendering determinism or precision.

A generated test is reviewed before it lands. That makes a recall/precision tradeoff
affordable *here* that is not affordable in the engine. That is the whole argument — it is
not that the engine is wrong.

## Input per record

The full quire record, the step-2 grounding attempt (including its refusal reason), the FR
file, the sibling ACs, and the located symbol if there is one.

## It may propose exactly one of three things

1. **Reclassify** into one of the 8 generatable families, with a proposed domain,
   precondition and oracle, each carrying at least one `file:line` citation. → a property
   test, plus a finding recording that an LLM read it.

   Worth looking for: a criterion phrased about one concrete input that actually holds for
   a whole class ("Repeated `--types` accumulate into an ordered list" is `ordering`, not an
   example); an error message criterion that is really `error-case` over a negative domain.

2. **Witness** — the criterion genuinely describes a single case, or a small closed set of
   them ("`version`, `--version`, and `-v` each print the package version" is a 3-case
   witness). → an example-based test, `Type: Unit`, with a finding, and **explicitly not
   counted as property coverage** in the census.

3. **Nothing** — no test; the reason becomes a `medium` finding. It records what could not
   be settled, and is not a judgement on the criterion (CON-1).

## It must refuse to

- **Reword, split, or recommend editing a criterion**, or emit anything that reads as
  pressure on the author. A concrete criterion is a legitimate criterion (CON-1).
- **Invent an oracle** absent from both the FR and the code. No tautologies, no
  `assert x is not None`, no isinstance-only checks, no assertionless tests. The
  weak-assertion heuristics in `gap-analysis/references/step-5-semantic-review.md` apply to
  generated tests too.
- **Propose a new regex, lexicon entry, or idiom widening for quire-rs.** The ceiling is
  accepted. Do not re-propose the two deleted widenings.
- **Write a framework name into any spec artifact** (CON-2).
- **Alter or synthesize a `row_id`.** Every emitted tag must be a `row_id` present in the
  classification output.
- **Present its output as deterministic.** Every test from this pass carries
  `origin=llm-second-pass` and a finding; the reviewer must be able to tell an LLM read it.

## Marking

Every product of this pass:

```
spec-correctness: row=FR-005-AC-2 property=example extraction=not-extractable origin=llm-second-pass confidence=medium
```

The test is written to the normal path and **runs** like any other. Its finding in the
review artifact is what marks it as needing a read.

`extraction: candidate` records use the identical mechanism with `origin=regex-candidate`
and no `confidence` key — they came from the deterministic pass, not from this one.

## Calibrating confidence

- `high` — the oracle is a direct quote of a `SHALL` predicate and the symbol was located.
- `medium` — the oracle was assembled from `## Behavior` plus the code, and reads as a fair
  restatement.
- `low` — the oracle is a plausible reading among more than one. Say what the alternative
  reading was, in the finding's summary.

Anything below `low` is proposal 3: nothing.
