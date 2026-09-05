# Corpus measurement, Track A Wave 4

**Advisory. Report only.** No corpus repository was edited, nothing was
normalized or migrated, and no schema was weakened to lower a count. Findings
here feed a later normalization campaign that this ticket does not perform.

Population identifier: `pop:cafc3ffdf1b73a29`
Corpus identifier: `ix-workspace-2026-09-04`
Run duration: 40.0 s

Every figure printed below is bound in `figure-index.json` to the artifact and
field it came from, and recomputed from it. A figure that has drifted from its
artifact is reported rather than corrected, because the drift is the thing
worth knowing.

## What was measured

| | |
|---|---|
| Repositories enumerated | **251** |
| Spec documents | **7,587** |
| Modules resolved | **10** |
| Declared types | **90** (61 object, 29 artifact) |

The exclusion vocabulary was `node_modules`, `.git`, `dist`, `target`,
`.venv`, `__pycache__`. That vocabulary is an authored input and is recorded
verbatim beside every count, because the same workspace yields 331,702, 24,643
or 7,587 documents depending on it. A rate whose population is unstated is not
a measurement.

78 candidates were excluded, each under a named rule: `not-applicable` 36,
`no-spec-directory` 23, `git-link-file` 16, `excluded-directory` 2, `symlink` 1.
The 16 `git-link-file` exclusions are worktrees created during this campaign;
counting them would have reported those repositories twice, each under a
different state.

**3 repositories were dirty and 0 were unstable at census time.** Both counts
are published here beside the rates rather than left in the enumeration record.

## Document states

| State | Documents |
|---|---|
| measured | **7,501** |
| out-of-model | 86 |
| unreadable | 0 |
| contested | 0 |

The states are asserted exhaustive and to sum to the enumerated count. The
assertion throws rather than returning a flag: a census that has lost documents
must not be able to publish a rate.

`out-of-model` keeps its two reasons apart — 71 documents declare no type, 15
declare a type no module knows. Those are different facts. The second group
includes `spec-analysis` beside `SpecAnalysis`, and `test-matrix` and `review`
beside their declared PascalCase forms; type resolution is case-sensitive by
contract, so these are corpus findings rather than parser noise.

No type is declared by two modules, so nothing is contested.

## Structural conformance

**7,370 / 7,501 documents = 98.3 %**

Unit: documents. Method: `engine-structural-v1`. The engine decides
conformance and this records what it said; nothing here re-implements a rule.
`could-not-run` was 0, and would have left both sides of the rate rather than
counting as a failure — "the engine rejected this document" and "the engine
never reached this document" are different facts.

## Properties-form census

**1 / 155 applicable documents = 0.6 %**

Unit: documents. Method: `properties-form-census-v1`. Reported separately from
the structural rate and never summed with it: the two count different
populations by different methods, and a combined figure would describe neither.

| Form | Documents |
|---|---|
| not-applicable (no `## Properties`) | 7,432 |
| free-column-table (legacy) | 154 |
| typed-table | **1** |

The denominator is 155, not 7,587. A document with no Properties heading is
neither conforming nor legacy, and counting it on either side would report a
rate about documents the rule does not reach.

All 154 legacy-form documents are **advisory** findings, not failures. Every
measured module declares `legacy_forms: warning`, so failing a document for a
form its own module admits would be this measurement inventing a rule.

**Bad rule or bad corpus: sampled 4, 0 rule, 4 real.** Three documents in
`agent-config-models` carry `| Column | Type | Constraints |` and one in
`agent-cli-daemon` carries `| Name | Type | Required | Description |`. None
carries the required `Field | Type | Multiplicity | Constraints`. The
classifier reads them correctly; the documents genuinely use other column sets.

The field-level dimension — resolving a `Type` token, a multiplicity, a
constraint keyword — is recorded `could-not-run` for every document, citing
`agent-ix/quire-rs#392`. That is not a pass and not a failure; it is a check
that did not happen.

## Finding partition

300 findings, every one classified with an owner and a disposition.

| Class | Findings |
|---|---|
| missing-structure | **292** |
| malformed-document | 7 |
| stale-module | 1 |
| contract-defect | 0 |
| legitimate-undeclared-value | 0 |
| unsupported-representation | 0 |
| tool-defect | 0 |
| **unknown** | **0** |

`unknown` and `undispositioned` are both **0**. They are published as their own
figures rather than inferred from a total, because they are the honest measure
of how far the partition got.

Each class was settled by reading sampled documents, and the sample is recorded
beside the rule in `src/measurement/classify-rules.ts`:

- **missing-structure** — `spec/tests.md` documents with no `test_cases` or
  `functional_coverage` table at all. Sampled `agent-cli-daemon` and
  `agent-config-cookiecutter`: absent, not present in an unreadable form.
- **malformed-document** — tables whose declared column set does not match the
  archetype, and `Traces To` cells holding an em dash where an id is required.
  Sampled `agent-duncan`, `catalog-service`, `cloud-manager-ui-services`.
- **stale-module** — `AssuranceProfile` documents in `quire-rs` carrying a
  `concern` property the pinned archetype does not admit. Sampled `AP-201`.

None is a rule defect, so no rule was widened. All are deferred to the corpus
normalization campaign with the owning repository named.

## What this measurement could not establish

The tool-defect ledger declares five entries, each citing a repository and
issue. One covers the entire population:

**`agent-ix/quire-rs#405` — the pinned module set is not closed.**
`IX_FILAMENT_MODULES_PATH` adds to the default install root rather than
replacing it, and resolution is first-wins. Every module in this run is
therefore reported twice, and which copy answered a given document is not
determined by the pin.

**The verdicts above stand. Their attribution to the pinned contract revisions
does not.** That is stated here rather than implied away, because a rate that
cannot be tied to the contracts it claims to measure is a number, not a
measurement — and nothing in the engine's output says so on its own.

The other four entries — `quire-rs#402`, `quire-rs#403`,
`spec-artifacts-process#81`, `quoin#347` — matched no finding in this run and
are reported as unmatched. An entry covering nothing is fixed, mis-scoped, or
was never real; all three are worth knowing.

## Reproducibility

Determinism is asserted over a committed fixture corpus under
`tests/fixtures/corpus-measurement/`, not over this workspace. A determinism
claim about one developer's machine cannot be reproduced by anyone else, and it
degrades whenever that workspace changes — which, for a workspace this campaign
committed to all day, is continuously. The three dirty repositories above are
that hazard in miniature.
