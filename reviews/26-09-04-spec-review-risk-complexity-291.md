---
id: SR-146
title: "Risk and complexity review of the quoin#291 corpus measurement requirements"
type: SpecReview
analysis: risk-complexity
scope: "US-022, FR-084..FR-092, NFR-021..NFR-023"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-087"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-088"
    type: "reviews"
  - target: "ix://agent-ix/quoin/FR-089"
    type: "reviews"
  - target: "ix://agent-ix/quoin/NFR-022"
    type: "reviews"
---

# SR-146: Risk and complexity review of the quoin#291 corpus measurement requirements

## Summary

Thirteen requirement-bearing documents were scored on technical risk and volatility —
US-022, FR-084..FR-092 and NFR-021..NFR-023, the artifacts added by commit `2e5d704`
for `agent-ix/quoin#291`. No spec artifact was edited and no corpus repository was
touched; every number below was obtained by reading the workspace read-only.

The ticket is advisory and report-only, so the blast radius of a defect is not a broken
build. It is a wrong published number — and this programme has already withdrawn three
of those. Under that framing the risk is concentrated in one requirement and one data
artifact.

The requirement is **FR-087**. It asks a single generic evaluator to build, from ten
modules' declared mappings, the same record those modules' own code builds. Two of the
modules ship that code: `spec-artifacts-iso/tests/support/reference_mapping.py` is 871
lines and `spec-artifacts-app/tests/support/reference_mapping.py` is 938 lines. Neither
is named anywhere in the requirement set, and no acceptance criterion requires the
generic evaluator to agree with either of them on any input. FR-087-AC-7 demonstrates
that headings, column lists and row-id patterns come from the module's declaration —
the declarative half. The half that carries the disagreement risk is the `parses:`
block of `mappings.yaml`, which states six rules (`story`, `verification`, `row-id`,
`index-entry`, `log-entry`, `success-criterion`) as English paragraphs. A second
implementation of a prose rule is not derived from the declaration; it is a
reimplementation, and it can be wrong while reporting `pass`. That is the exact
failure mode the ticket exists to avoid, moved from the corpus into the instrument.

The data artifact is the **classification ledger** of FR-089 and FR-091. It is
hand-maintained, it decides the two figures FR-089 makes headline (`unknown` count,
`undispositioned` count), and the requirements never say what a failure's identity is,
how a "failure group" entry matches, or what happens to an entry that matches nothing.
Every unstated case drifts the same way — toward a smaller `unknown` count than the
evidence supports.

Volatility is high across the module-facing requirements for a reason external to the
spec: the contracts being measured are days old and mostly unreleased. The mappings
declaration this whole check depends on is **absent from the catalog pin**
(`git cat-file -e v0.18.0:spec_artifacts_iso/mappings.yaml` fails); it exists only on
`spec-artifacts-iso` `main` at `6686f11`, six commits past the last tag, committed
2026-09-03. `spec-artifacts-app` and `spec-objects-business` were both committed
2026-09-04. FR-085 handles this correctly by measuring declared revisions and recording
pin divergence, but nothing in the set bounds how stale a published rate may be when
the promotion decision consumes it.

Findings FND-1460..FND-1469 are this review. They do not restate SR-145's vocabulary
defects (FND-1450..FND-1459) or SR-142's grammar defects; where a risk finding depends
on one of those it says so. Two findings overlap SR-144's failure-domain pass and are
reconciled under "Failure-domain gaps" below.

## Verdict

CONDITIONAL. Nothing here says the requirement set should be rewritten, and none of
these findings blocks tasking FR-084, FR-086, FR-090, FR-091 or FR-092 — those are
low-risk plumbing and can start now.

Three findings should be resolved before FR-087 is tasked, because each of them decides
whether the number this campaign publishes is trustworthy: FND-1460 (no oracle binding
the generic evaluator to the modules' own mappers), FND-1461 (the mapping semantics
that matter are prose), and FND-1464 (the ledger can shrink the `unknown` headline with
nothing checking it). FND-1463 should be resolved before FR-088 is tasked, because
FR-088-CON-2 as written cannot be satisfied by any module in the set.

The cheapest fix for the first two is already committed and free: `spec-artifacts-iso`
ships ten skeletons and ten golden records (`spec_artifacts_iso/skeletons/*.md`,
`spec_artifacts_iso/examples/*.record.json`), and its `mappings.yaml` declares those
records reproducible byte-for-byte. Requiring the generic evaluator to reproduce them
turns a silent divergence into a failing test before a single corpus number is
published. It is narrow coverage — ten ideal documents, not the corpus — but it is a
floor where there is currently none.

## Risk Register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| US-022 | Medium | Medium | Story is stable; its Options name the catalog route, which is blocked by `agent-ix/quoin#347`, leaving the generic-evaluator route as the only live one | None needed at story level; the risk lands on FR-087 |
| FR-084 | Low | High | Population is "every `*.md` under each retained repository" filtered by an exclusion vocabulary no artifact declares; the live spread is 331,702 / 24,643 / 7,587 documents | Declare the exclusion vocabulary as a spec artifact before tasking (FND-1465) |
| FR-085 | Medium | High | Measures declared revisions, not catalog pins; the declaration under test is unreleased and six commits past the last tag | Keep the pin-divergence record as a headline; re-run at the revision actually promoted (FND-1462) |
| FR-086 | Low | Medium | State assignment is simple; the type vocabulary it resolves against moves whenever a module ships | Covered by FR-085's pinning; no separate mitigation |
| FR-087 | High | High | Second implementation of prose-declared mapping semantics, competing with two shipped reference mappers totalling 1,809 lines; kind list frozen from today's two modules | Differential test against the committed golden records before publishing any rate; treat a disagreement as instrument failure (FND-1460, FND-1461, FND-1469) |
| FR-088 | High | Medium | Requires vocabularies (constraint keywords, multiplicity forms, kernel scalars) that no module in the set declares; the only declarations live in quoin's own `semantic-core` schemas | Resolve FR-088-CON-2 — either the modules publish the vocabularies or the constraint is restated (FND-1463) |
| FR-089 | Medium | High | Hand-maintained ledger with no failure identity, no group-match rule and no unmatched-entry report; it directly moves the two headline counts | Key entries on a failure digest; publish unmatched entries and per-group match counts (FND-1464) |
| FR-090 | Low | Low | Arithmetic and report shape over records other requirements produce | The undeclared margin is SR-145 FND-1456; no new risk mitigation |
| FR-091 | Low | High | Ledger of live tool defects; the four defects it names are expected to be fixed while the campaign runs, changing coverage between runs | Record each entry's resolution state so a fixed defect stops suppressing findings silently |
| FR-092 | Low | Low | Read-only and exit-0 discipline; well-bounded and independently testable | None; task it early as the safety harness for every other run |
| NFR-021 | Medium | Medium | Byte-identical artifacts scoped to "a clean checkout", while FR-084 explicitly retains dirty repositories and measures their working trees; 3 of 251 corpus repositories are dirty now | State the reproducible sub-population, or read corpus documents from the object store as FR-085 does (FND-1467) |
| NFR-022 | Medium | High | 15-minute and 4 GiB budget with no declared population, over a corpus whose size is undetermined by a factor of 13, including a JSON Schema validation per measured document | Spike one repository, extrapolate, restate the threshold against the declared population before committing it (FND-1466) |
| NFR-023 | Medium | Low | Requires an automated recomputation of every printed figure from the artifacts — real work, but stable and independent of the modules | Task it with FR-090; it is the check that would have caught the three withdrawn figures |

## Top hazards

1. **FR-087 has no oracle.** A generic evaluator that silently disagrees with a
   module's own reference mapper publishes a wrong corpus rate that looks exactly like
   a right one. FND-1460, FND-1461.
2. **The FR-089 ledger is unbound hand-maintained data.** Every case the requirements
   leave unstated drifts toward a smaller `unknown` count. FND-1464.
3. **FR-088-CON-2 cannot be satisfied.** No module declares the constraint keyword or
   multiplicity vocabularies the constraint forbids the measurement from holding.
   FND-1463.
4. **The measured population is undetermined.** FR-084's exclusion vocabulary is an
   undeclared input that swings the corpus between 7,587 and 331,702 documents, and
   NFR-022's budget is stated against none of them. FND-1465, FND-1466.
5. **The contract under measurement is days old and unreleased.** The declaration FR-087
   evaluates does not exist at the catalog pin. FND-1462.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-1460 | high | No acceptance criterion binds the generic evaluator of FR-087 to a module's own oracle, although two modules ship one (`spec-artifacts-iso/tests/support/reference_mapping.py`, 871 lines; `spec-artifacts-app/tests/support/reference_mapping.py`, 938 lines) and `spec-artifacts-iso` commits ten golden records its `mappings.yaml` declares byte-for-byte reproducible; FR-087-AC-7 demonstrates only that headings, columns and row-id patterns are derived, which is the half that cannot disagree. | FR-087, US-022 |
| FND-1461 | high | The mapping semantics that carry the disagreement risk are declared in English prose, not as data: `spec-artifacts-iso/spec_artifacts_iso/mappings.yaml` states six parse rules (`story`, `verification`, `row-id`, `index-entry`, `log-entry`, `success-criterion`) as paragraphs, so FR-087's "derive from the module's own declaration rather than from a vocabulary compiled into the measurement" is unreachable for exactly the rules whose reimplementation can be wrong while reporting `pass`. | FR-087 |
| FND-1464 | high | The FR-089 classification ledger is unbound hand-maintained data that moves the campaign's two headline counts: the requirements define no identity for a failure, no matching rule for the "failure group" the Inputs permit, and no report of a ledger entry that matched nothing or that matched thousands, so a stale or over-broad entry shrinks the `unknown` and `undispositioned` figures with no acceptance criterion able to detect it. | FR-089, FR-091, FR-090 |
| FND-1463 | high | FR-088-CON-2 requires the constraint keyword vocabulary to be "the one the resolved module set declares, never a copy held by the measurement", but no module declares one: a search across all ten module data directories finds no constraint-keyword or multiplicity vocabulary, and the only declarations are quoin's own `src/semantic/schemas/semantic-core/ConstraintKeyword.json` and `Multiplicity.json` — that is, the copy the constraint forbids. FR-088's "declared multiplicity forms" and "declared kernel scalars" have the same problem. | FR-088 |
| FND-1462 | high | The contract FR-087 measures is unreleased and days old: `mappings.yaml` is absent from the catalog pin `v0.18.0` and exists only on `spec-artifacts-iso` `main` at `6686f11` (2026-09-03, six commits past the tag), while `spec-artifacts-app` and `spec-objects-business` were committed 2026-09-04; FR-085 records pin divergence but nothing bounds how stale a published rate may be when the promotion gate reads it. | FR-085, FR-087, NFR-021 |
| FND-1465 | medium | FR-084's population turns on "a declared exclusion vocabulary" that no artifact in the set declares, and the enumeration rule is every `*.md` in the repository rather than under `spec/`; measured on the live workspace the same rule yields 331,702 documents unfiltered, 24,643 excluding `node_modules`, `target`, `dist` and dot-directories, and 7,587 under `spec/` alone, across 251 qualifying repositories. | FR-084, NFR-022 |
| FND-1466 | medium | NFR-022 commits a 15-minute wall-clock and 4 GiB memory budget without naming the population it applies to, over a corpus undetermined by a factor of 13 (FND-1465) and a workload including a JSON Schema validation per measured document against a schema set of 58 files in `spec-artifacts-iso` alone; no spike is required before the threshold is committed, and FR-089 and FR-090 need the evaluation records retained to partition and cross-tabulate them. | NFR-022, FR-087, FR-090 |
| FND-1467 | medium | NFR-021 scopes byte-identical reproducibility to "a clean checkout of each pinned repository", while FR-084 explicitly retains a dirty repository with `clean: false` and measures its working tree; 3 of the 251 qualifying repositories are dirty in the live workspace, and no requirement says a rate computed over them is outside the reproducibility claim — unlike FR-085, which reads module content from the object store precisely so a dirty checkout cannot change what was measured. | NFR-021, FR-084, FR-085 |
| FND-1468 | medium | Mapping coverage is far narrower than "the corpus" and the set never names the ratio: only two of the ten modules publish a `mappings.yaml`, covering twelve types, so a `type:` census under `*/spec` of the live workspace puts roughly 7,240 documents inside the mapping check and roughly 1,685 outside it as `no-mapping-for-declared-type` (SpecReview 555, ADR 404, TestMatrix 229, Task 163, Review 90, MeasurementPlan 49, Plan 42, Standard 41, plus the assurance types); FR-090 prints the `could-not-run` count beside the rate, so this is not a hole, but it is the single largest determinant of what the headline number means. | FR-087, FR-090, FR-085 |
| FND-1469 | medium | FR-087's nine mapping kinds are a vocabulary compiled into the requirement, contradicting the clause beside it: the list is exactly the union of the eight kinds `spec-artifacts-iso` declares in its `kinds:` block (no `sysml-fence`) and the six `spec-artifacts-app` declares (adds `sysml-fence`, omits `table`, `list` and `token`), so it is today's two modules frozen into normative text; the `could-not-run` escape hatch bounds the damage, but the enumeration reads as a contract and will be cited as one. | FR-087, FR-085 |

## Evidence

Every figure in this review was obtained read-only from the live workspace at
`/home/peter/dev` on 2026-09-04, and each is stated with its unit and its population.

**Reference mappers and golden records.** `wc -l` over
`spec-artifacts-iso/tests/support/reference_mapping.py` and
`spec-artifacts-app/tests/support/reference_mapping.py` gives 871 and 938 lines, 1,809
together. A search for `reference_mapping*.py` across the workspace finds these two and
no others, so eight of the ten modules ship no oracle at all. `spec-artifacts-iso`
carries ten skeletons, ten `examples/*.record.json` golden records and 58 schema files;
its `mappings.yaml` header states the golden records are reproducible byte-for-byte
from the skeletons.

**Mapping declarations.** `mappings.yaml` exists in `spec-artifacts-iso` and
`spec-artifacts-app` only. Its `kinds:` blocks declare eight and six kinds respectively,
whose union is the nine of FR-087. Kind usage counts in `spec-artifacts-iso` are
frontmatter 62, section 47, provenance 10, typed-table 4, table 2, list 2, ocl-clause 1,
token 1; in `spec-artifacts-app`, section 12, frontmatter 10, typed-table 10,
sysml-fence 2, ocl-clause 2, provenance 2. The `parses:` block of `spec-artifacts-iso`
holds six rules, each a prose paragraph. Models declared: ten in
`spec-artifacts-iso` (FR, NFR, StR, US, IT, TC, master-requirements, index, log,
Glossary) and two in `spec-artifacts-app` (ApplicationSpec, MasterRequirements).

**Revisions.** `git rev-parse v0.18.0` in `spec-artifacts-iso` resolves, but
`git cat-file -e v0.18.0:spec_artifacts_iso/mappings.yaml` reports the path absent;
`git rev-list v0.18.0..HEAD --count` is 6 and HEAD is `6686f11` dated 2026-09-03. Last
commit dates for the ten catalog modules: `spec-artifacts-app` 2026-09-04,
`spec-objects-business` 2026-09-04, `spec-artifacts-iso` 2026-09-03,
`engineering-assurance` 2026-09-01, `spec-artifacts-process` 2026-08-27,
`spec-objects-security` 2026-08-19, `spec-objects-safety` 2026-08-18, and
`spec-objects-architecture`, `spec-objects-enterprise` and `spec-objects-operational`
2026-08-08. `spec-artifacts-process` is checked out on
`epic/264-assurance-integration`, not `main`.

**Corpus population.** Directories under `/home/peter/dev` containing both `.git/` and
`spec/`: 251 of 311. `*.md` files beneath them: 331,702 unfiltered; 24,643 excluding
`node_modules`, `target`, `dist`, `venv` and dot-directories; 7,587 counting only
`spec/`. Working trees dirty: 3 of 251.

**Type census.** `type:` frontmatter values across `*/spec/**/*.md`, counted by value:
FR 2,863, index 1,173, US 1,055, NFR 809, SpecReview 555, StR 543, ADR 404, log 285,
master-requirements 249, TestMatrix 229, IT 184, Task 163, Review 90, TC 67,
MeasurementPlan 49, Plan 42, Standard 41, AssuranceProfile 20, SuiteRegistry 20,
ComponentAssuranceContract 20, AssuranceArgument 20, ArchitectureDescription 20,
MasterRequirements 14, discovery 7, interface 5. Partitioning those against the twelve
mapped types gives the roughly 7,240 / 1,685 split of FND-1468.

**Declared vocabularies.** A search for constraint-keyword or multiplicity declarations
across `spec-objects-*/` and `spec-artifacts-*/` YAML and JSON returns only
`spec-artifacts-app/spec_artifacts_app/mappings.yaml` and its schema, neither of which
declares a keyword set. `spec-objects-business/spec_objects_business/` contains
`manifest.yaml`, `schemas/` and `skeletons/` and no vocabulary file. The closed
constraint keyword set and the multiplicity model are declared in
`quoin/src/semantic/schemas/semantic-core/ConstraintKeyword.json` and
`Multiplicity.json`.

## Failure-domain gaps

The failure-domain analysis for this set is SR-144
(`reviews/26-09-04-spec-review-failure-domain-291.md`), verdict FAIL on seven high
findings, FND-1440..FND-1449. Its results raise rather than lower the scores above:
FND-1440 and FND-1441 (no symlink, cycle, nested-repository or submodule rule) make
FR-084's population less determined than FND-1465 alone shows, and FND-1445 (no fault
isolation or time bound at the mapping-evaluation boundary) means one hanging document
can end a run that NFR-022 budgets at 15 minutes.

Two findings overlap and should be resolved by one edit each rather than counted twice.
SR-144 FND-1447 and this review's FND-1464 are the same defect in the FR-089 ledger
seen from two sides — SR-144 states the missing join key and stale-entry rule, this
review states the consequence for the `unknown` and `undispositioned` headline counts
and adds the unbounded "failure group" match. SR-144 FND-1442 and this review's
FND-1467 are both the working-tree-versus-commit identity gap in FR-084; SR-144 states
it as an unpinned read, this review states it as the contradiction with NFR-021's
declared scope and quantifies it at 3 dirty repositories of 251.

The four risk findings with no failure-domain counterpart are FND-1460, FND-1461,
FND-1463 and FND-1469 — all of them about the instrument's fidelity to the modules'
own declarations rather than about a topology or identity gap.

## Notes

The skill's deliverable template records the register at `spec/analysis/risk-register.md`.
It is reproduced here instead because this review was scoped read-only over the spec
artifacts; promoting the register into `spec/analysis/` is a separate, spec-editing
change.

Structural validation: `quire validate --scope
/home/peter/dev/quoin/.worktrees/291-corpus-measurement
"reviews/26-09-04-spec-review-risk-complexity-291.md"` reports this document valid.
