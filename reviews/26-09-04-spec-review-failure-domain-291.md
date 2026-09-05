---
id: SR-144
title: "Failure-domain review of the quoin#291 corpus measurement contracts"
type: SpecReview
analysis: failure-domain
scope: "spec/usecase/US-022, spec/functional/FR-084..FR-092, spec/non-functional/NFR-021..NFR-023"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/US-022"
    type: "reviews"
---

# SR-144: Failure-domain review of the quoin#291 corpus measurement contracts

## Summary

The review challenged the enumeration topology, the identity of a "document", the pin that is
supposed to name what was measured, the trust boundary at module-declared mapping rules, the two
external ledgers, and the read-only claim of the run. The artifacts are strong on state
exhaustiveness and on rate provenance, but ten failure modes are unstated, and four of them are
already realised by the actual workspace the measurement is specified to enumerate: a symlinked
duplicate of a repository, a checked-out submodule of deliberately malformed fixtures, mutable
module revisions, and Git plumbing that writes inside the repositories the spec forbids writing to.

## Verdict

**FAIL** — seven `high` findings; the enumeration topology, the measured-bytes pin and the
mapping-evaluation trust boundary are undefined, and each can move a published corpus rate without
any requirement detecting it.

## Findings

| ID       | Severity | Summary                                                                                                                              | Refs                                                             | Escape Cause        |
| -------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------- | ------------------- |
| FND-1440 | high     | Enumeration has no symlink rule: a symlinked repository is counted twice and a directory cycle does not terminate.                     | FR-084 Behavior; FR-084-AC-6; FR-086-AC-5; FR-086-AC-6; NFR-022   | missing-requirement |
| FND-1441 | high     | A nested repository or submodule is neither a repository nor excluded; its documents are counted under the parent's origin and commit. | FR-084 Behavior; FR-084-AC-2; FR-086 Outputs                       | missing-requirement |
| FND-1442 | high     | Documents are read from the working tree while the pin records the commit, and nothing re-verifies the tree after the run.             | FR-084 Behavior; FR-085 Behavior; NFR-021-AC-1                     | wrong-requirement   |
| FND-1443 | high     | A declared module revision may be a mutable ref; nothing requires the population identifier to carry the resolved immutable commit.    | FR-085 Inputs; FR-085-AC-1; FR-090-AC-2; NFR-021-AC-1              | missing-requirement |
| FND-1444 | high     | Disagreement between manifest, mappings and schemas is undefined; a declared type with no schema passes validation vacuously.          | FR-085 Behavior; FR-087 Behavior; FR-087-AC-1                      | missing-requirement |
| FND-1445 | high     | No fault isolation or time bound at the mapping-evaluation trust boundary; one hang or crash aborts the whole census.                  | FR-087 Behavior; FR-087-AC-4; FR-092-AC-1; NFR-022-AC-1            | missing-requirement |
| FND-1446 | high     | Document, row and repository are mixed as denominators; FR-088 never says when a failing row makes its document fail.                  | FR-088 Behavior; FR-088-AC-3; FR-090 Outputs; FR-091-AC-4          | missing-requirement |
| FND-1447 | medium   | The partition ledger has no declared join key and no stale-entry rule, so it can silently suppress a new, different failure.           | FR-089 Inputs; FR-089-AC-6; FR-091-CON-2                           | missing-requirement |
| FND-1448 | medium   | A contested type has no place in the by-type breakdown and never reaches the failure partition, so nobody owns it.                     | FR-086 Behavior; FR-090-AC-7; FR-089-AC-1                          | missing-requirement |
| FND-1449 | medium   | "SHALL NOT write to, or inside, any enumerated repository" is unsatisfiable: reading a commit or a status writes inside `.git`.        | FR-084-CON-1; FR-092-AC-2; FR-092 Behavior; NFR-022-AC-3           | wrong-requirement   |

## Detail

### FND-1440 — enumeration topology: symlinks alias and cycle

FR-084's only exclusions are dot-prefixed segments and a declared directory vocabulary. Nothing
says whether the walk follows symbolic links, and nothing bounds it if it does.

Both halves are live in the workspace the measurement is specified over:

- `/home/peter/dev/filament` is a symlink to `/home/peter/dev/spec-editor-app-filament-foundation`.
  Following links, that repository is enumerated twice under two names, with the same `origin` and
  the same `commit`. Every one of its documents is counted twice, in the population, in the
  numerator, and in the by-repository breakdown — which also silently violates FR-086-AC-6 ("no
  document appears in more than one state record") unless document identity is defined to defeat
  it, and it is not: FR-086's record key is `repository` plus repository-relative path, which two
  aliases of one repository do not share.
- Many repositories carry `.venv-*/lib64 -> lib`; a link back to an ancestor directory makes the
  walk non-terminating, which is the only way NFR-022's 15-minute bound fails without a single
  slow document.

FR-084 needs a stated link policy — the natural one being: do not traverse symbolic links, record
each skipped link as an `excluded` entry with the link rule, and key document identity on the
resolved real path so an alias reached by any route is counted once.

### FND-1441 — a repository inside a repository

FR-084 recognises a repository by `.git` plus `spec/`, and explicitly refuses a directory whose
`.git` is a *file* so that worktrees are not double-counted. A submodule's `.git` is also a file.
The rule therefore rejects submodules as repositories, and no other rule excludes their contents,
so their documents are enumerated as documents *of the parent repository*, attributed to the
parent's `origin` and the parent's `commit`.

This repository does exactly that: `/home/peter/dev/quoin/.gitmodules` declares the submodule
`corpus` → `agent-ix/qa-corpus`, checked out at `corpus/` with a `.git` file and 338 Markdown
files. `qa-corpus` is the QA fixture corpus — documents that are *deliberately* malformed. Under
FR-084 as written, several hundred intentionally broken fixtures enter the governed-corpus
population as Quoin's own documents, at Quoin's commit, and their failures land in FR-089's
partition with Quoin named as owner. The same shape recurs in `quire-rs` and
`engineering-assurance`, both of which declare submodules.

Three things are missing and none of them is implied: whether a nested repository is its own
population member or is excluded, that a parent's enumeration stops at a nested repository
boundary either way, and that the exclusion — like every other — is recorded with its rule
(FR-084-CON-2). The `clean` flag has the same contamination: a dirty submodule makes the parent
tree dirty, so FR-084-AC-4 records `clean: false` against a repository whose own documents did not
change.

### FND-1442 — the pin does not identify the bytes measured

FR-085 is careful: module content is read *from the object store at the declared revision*, so a
dirty checkout cannot change what was measured. FR-084 is the opposite and does not say so: it
enumerates `*.md` files on disk, records the `commit` as the pin, and records `clean: false` when
the tree disagrees with that commit — but retains the repository and measures the tree anyway.

The recorded pin therefore names content that was, for any dirty repository, not the content
measured. NFR-021-AC-1 ("two runs over the same recorded corpus and module revisions produce
equal digests") cannot hold for those repositories, because the recorded revisions do not
determine the input. NFR-021's own scope quietly concedes this by assuming "a clean checkout of
each pinned repository", which is a precondition no requirement establishes and which the workspace
does not satisfy.

Concurrent mutation is the same defect over time and is equally unstated. `clean` is sampled once
at enumeration; documents are read afterwards, over a run NFR-022 budgets at up to 15 minutes. A
branch switch, a rebase, a `git gc`, or an editor save mid-run yields a census assembled from two
different trees and published as one population, with no requirement able to detect it. What is
missing is a post-run re-verification: re-read each repository's `HEAD` and cleanliness at the end
of the run and report any repository that moved, and a stated rule for whether a dirty repository
is measured from the tree (and the pin recorded as tree-not-commit) or refused.

### FND-1443 — a revision that is a tag that later moves

FR-085's input is "a revision" per module, and its outputs record the *requested* revision and the
*resolved* commit. Nothing requires the requested revision to be immutable, and nothing requires
the reproducibility contract to be expressed over the resolved commit rather than the request.

So: declare `v0.3.0`, resolve it, publish. The tag is force-moved a week later — normal for this
ecosystem, where module tags are cut and re-cut during a wave. A re-run "at the same declared
module set" now measures different schemas and different mappings, produces different digests, and
FR-090-AC-2's population identifier — "names the corpus revision, the module revisions" — reads
identically in both reports because it does not say *which* revision it names. NFR-021-AC-1 fails
for a reason nobody can see, and the promotion gate compares two numbers about two different
contracts.

FR-090 should require the population identifier to carry the resolved commit object id, and FR-085
should require a re-run to report a divergence when a declared ref resolves to a commit other than
the one a prior recorded population names. FR-085-AC-6's `default-modules.yaml` comparison has the
same latent ambiguity: comparing a ref to a ref rather than a commit to a commit will report
agreement across a moved tag.

### FND-1444 — the manifest, the mappings and the schemas can disagree

FR-085 records the manifest, the declared types, the JSON Schemas and the mappings declaration by
digest, but never requires them to be mutually consistent, and defines no outcome when they are
not. FR-087 covers exactly one of the four disagreements:

| Disagreement | Defined? |
| --- | --- |
| Declared type with a schema and no mapping | Yes — `could-not-run` / `no-mapping-for-declared-type` |
| Mapping entry for a type the manifest does not declare | No |
| Declared type with a mapping and **no schema** | No |
| Mapping or manifest naming a schema file absent at the revision | No |

The third is the dangerous one. FR-087 says report `pass` "only when validation reports no error".
Validating against a schema that does not exist reports no error, so a whole module's documents
pass vacuously and lift the aggregate — the exact inflation mechanism FR-086's rationale is written
against, arriving through the module's declaration instead of through a silent exclusion. A type
with a mapping and no schema is a `could-not-run` (with reason `no-schema-for-declared-type`), and
a self-inconsistent module is a finding against the module, which is precisely the
`contract-defect` class FR-089 already provides.

### FND-1445 — the mapping evaluator is an untrusted extension point

FR-087 evaluates nine mapping kinds and derives every heading, column list and row-id *pattern*
from the module's own declaration (FR-087-AC-7) — that is, the measurement executes
module-supplied rules, including regular expressions, over 24,600 documents of arbitrary
authored text. That is a trust boundary, and the spec defines its failure behavior for exactly one
case: an *unimplemented* kind yields `could-not-run`.

Undefined, and each of them aborts the run as written:

- An implemented kind that raises on a particular document.
- A declared `row_id.pattern` that backtracks catastrophically on one long table row — the one way
  NFR-022's 15-minute bound is blown by a single document, and indistinguishable from a hang.
- A declared pattern that is not a valid regex at all.
- A pathological document (a multi-megabyte generated table) exhausting NFR-022's 4 GiB.

An aborted run contradicts FR-092-AC-1 (exit `0` whatever it finds) and loses the entire census to
one bad document. The checklist answer is the strict/resilient choice, and here it must be
resilient with a record: evaluation is isolated per document, a fault or a per-document timeout
yields `could-not-run` naming the fault, and the run continues — with an aggregate cap so that a
module whose declaration is systematically unevaluable is visible rather than absorbed.

### FND-1446 — document, row and repository as denominators

FR-088 mixes two units in one requirement without ever converting between them. It says "report
that **row** `fail`" (Type, Multiplicity, Constraints cells) and "report the **document** `fail`"
(both-representations) and "report the document `unsupported-representation`" and "report the
document `not-applicable`" — and never states the roll-up: is a document with one failing row of
forty a failing document? Are both `pass`? FR-088's own preamble promises the document is reported
as "`pass`, `fail` or `could-not-run`", while the ACs only ever demonstrate row verdicts.

The consequence lands in FR-090, whose whole purpose is that a rate names its unit. FR-090-AC-4
requires "the mapping rate and the representation rate" be published separately with their own
populations, but the representation rate's unit is undetermined by FR-088 — rows over rows, or
documents over documents, each of which is a different number and a different claim. FR-091-AC-4
shows the authors know the two units coexist, since it requires a tool-defect entry to report both
a document count and a row count.

Three units are in play across these artifacts — repository (FR-084, FR-090-AC-6), document
(FR-086, FR-087), row (FR-088, FR-091) — and only the repository one is unambiguous. FR-088 needs
an explicit row-to-document roll-up rule, and FR-090 needs the representation rate's unit named in
the requirement rather than left to the implementation.

### FND-1447 — the classification ledger can drift from the measurement

FR-089's ledger is an external declared input keyed to "each classified failure or failure group",
and FR-091's tool-defect ledger is keyed to a declared "scope". Neither says what a ledger entry
*matches on*. A failure is a tuple of document, check, schema keyword, instance path and line; a
ledger entry that matches on document alone will classify tomorrow's *different* failure in that
document as today's `accepted` — a real finding suppressed without anybody choosing to suppress it.
Line numbers move; instance paths do not; the spec picks neither.

The reverse drift is equally unstated: an entry matching *no* failure any more, because the corpus
was fixed or the module changed. FR-089 requires an unclassified failure to be reported (`unknown`,
`undispositioned`, both as headline counts), but imposes no symmetric obligation to report an
unmatched ledger entry, so a ledger silently rots and the campaign's claim to have dispositioned
everything decays with it. FR-091-CON-2 gets half of this right for tool defects ("SHALL NOT
suppress a finding outside the scope its entry declares") and FR-089 has no equivalent at all.

Needed: a declared match key stable across runs, a refusal of ambiguous or overlapping entries, and
an unmatched-entry count published beside the `unknown` and `undispositioned` counts.

### FND-1448 — a contested type has no owner and no row

FR-086 handles the contested-type case well as a *state*: two modules declaring one `type` gives
the document `unknown`, naming both, retained in every published output, never guessed. What
follows it is not handled.

An `unknown` document is not `measured`, so FR-087 and FR-088 never see it, so it produces no
failure, so FR-089 — where owners and dispositions are assigned — never receives it. A contested
type is a `contract-defect` in one of two modules by construction, and it is the one defect class
that leaves this measurement with nobody's name on it. The campaign's exit criterion is that every
failure has an owner; a contested type is a failure of the module set that the partition cannot
express.

FR-090-AC-7 then collides with it directly: "every declared type of every resolved module appears
in the by-type breakdown". A type declared by two modules is either one row (whose module
attribution is false) or two rows (which double-count any instance), and the requirement does not
say. The by-type key needs to be `(module, type)` rather than `type`, and a contested type needs a
route into the partition as its own class or as a declared module-level finding.

### FND-1449 — the read-only constraints are unsatisfiable as written

FR-084-CON-1 says enumeration "SHALL NOT write to, or inside, any enumerated repository", and
NFR-022 requires every corpus and module file to be opened read-only. But FR-084 must obtain each
repository's resolved commit and its cleanliness, and FR-085 must read blobs from the object store
— and Git's own plumbing writes inside `.git` while doing it: the status walk refreshes and
rewrites `.git/index` when stat data is stale, and object reads may take `.git/index.lock` or
`.git/objects` pack locks. A literal reading of FR-084-CON-1 forbids the mechanism FR-085 depends
on, and there is no way to satisfy both.

FR-092-AC-2 shows the authors mean something narrower and more checkable — "the Git *status* of
every enumerated corpus repository is byte-identical before and after" — that is, no tracked file,
no untracked file and no ref changes. FR-084-CON-1 and NFR-022-AC-3 should be restated in those
terms, with `.git` internal bookkeeping named as the explicit carve-out. Left as written, the
constraint is either untestable or it fails on the first repository.

The adjacent unstated failure is the lock itself: a concurrent Git operation in a corpus repository
makes a plumbing call fail with a lock error. There is no requirement saying whether that
repository becomes an `unresolved`/excluded population member with the failure recorded, or whether
the run aborts — and given FR-092-AC-6 makes an unresolvable *module set* a non-zero exit, the
asymmetry for corpus repositories should be stated rather than inferred.

## Recommended Additions

| Finding | Proposed home | Type |
| --- | --- | --- |
| FND-1440 | FR-084: symlink traversal policy, link exclusions recorded, document identity on resolved real path | FR |
| FND-1441 | FR-084: nested-repository and submodule policy, parent walk stops at the boundary | FR |
| FND-1442 | FR-084: measured-from-tree vs measured-from-commit stated; post-run pin and cleanliness re-verification | FR |
| FND-1443 | FR-085 / FR-090: population identifier carries resolved commit object ids; moved-ref divergence reported | FR |
| FND-1444 | FR-087: outcomes for mapping-without-manifest-type, type-without-schema, and absent schema file | FR |
| FND-1445 | FR-087 / NFR-022: per-document fault isolation and evaluation timeout, both yielding `could-not-run` | NFR |
| FND-1446 | FR-088 / FR-090: row-to-document roll-up rule and a named unit for the representation rate | FR |
| FND-1447 | FR-089: declared ledger match key, refusal of ambiguous entries, unmatched-entry count published | FR |
| FND-1448 | FR-086 / FR-089 / FR-090: contested types routed into the partition; by-type key is `(module, type)` | FR |
| FND-1449 | FR-084-CON-1 / NFR-022-AC-3: restate read-only as working-tree-and-refs invariance; lock-failure behavior | StR |

## Notes

Checked and found already handled, recorded so a later reviewer does not re-open them: state
exhaustiveness and mutual exclusion over documents (FR-086-CON-1, AC-5, AC-6); `could-not-run`
excluded from both numerator and denominator and counted beside every rate it was excluded from
(FR-087-CON-1, FR-090-AC-3); refusal to guess the owner of a contested type (FR-086 Behavior);
partitions with a denominator of one published rather than suppressed (FR-090-CON-2, AC-6); a
tool-defect claim without a citation refused (FR-089-AC-3, FR-091-AC-1); and the module-object-store
read that defeats a dirty module checkout (FR-085-AC-3) — which is exactly the rule FND-1442 asks
FR-084 to state for the corpus side.
