---
id: FR-153
title: "An analysis result counts toward an obligation only as a qualified passing record"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-009"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-152"
    type: "extends"
  - target: "ix://agent-ix/quoin/FR-151"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-115"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-040"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-116"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-032"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-093"
    type: "references"
  - target: "ix://agent-ix/quire-rs/FR-078"
    type: "requires"
  - target: "ix://agent-ix/engineering-assurance/FR-036"
    type: "references"
---

# FR-153: An analysis result counts toward an obligation only as a qualified passing record

## Description

An analysis result bound to an obligation by FR-152 is a retained record. This requirement states when that record
counts as coverage and trace for the obligation, per disposition of the report (FR-151). The counting rules
themselves are quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078) and the owner qualification
contract [Quoin FR-116](./FR-116-owner-qualified-quire-evidence.md); they are cited here and not restated. This
requirement adds only what is specific to an analysis entry.

## Inputs

`quoin matrix` and the derived retained-evidence view of `quoin assurance` use
the same optional, repeatable `--run-observation FILE` input. Each `FILE` is a
single UTF-8 JSON object with protocol `quoin.analysis-run-observation/v1`; it
selects exactly one canonical run record by its exact store-relative path and
may carry host observations for that record. JSON member types and empty-value
handling are:

| Member | Contract |
| --- | --- |
| `protocol` | Required non-empty string. Exactly `quoin.analysis-run-observation/v1`. |
| `record` | Required non-empty string. The exact canonical run-record path selected by FR-030, such as `runs/<suite>/<timestamp>.json`; it MUST be a relative canonical store path with no `.` or `..` path component and match one record in the selected store. It is a read-time locator, not a store field or pin. |
| `selected_head` | Optional string. The full Git object id of the source HEAD selected by the host for this qualification call: exactly 40 or 64 ASCII hexadecimal characters. Empty or malformed values leave FR-116's `HostObservations.selected_head` unavailable and the performed-revision premise `unevaluated`. |
| `checkout_sha` | Optional string. The full Git object id observed for the `run-package` checkout: exactly 40 or 64 ASCII hexadecimal characters. Empty or malformed values leave FR-116's `performed_revision` unavailable and that premise `unevaluated`. |
| `ci_association` | Optional object with exactly the required string members `system`, `project`, `run`, `attempt`, `job` and `invocation`. If any member is empty, execution association is `unevaluated`; otherwise the object supplies FR-116's CI `ExecutionAssociation`. |
| `signature_result` | Optional object containing one complete `engineering-assurance.signature-verify-result/v1` result for the commit that added the selected run and its FR-030 report projection, as defined by engineering-assurance FR-036 and FR-037. The outer host observation does not redefine that result schema; a missing, malformed or unverifiable result leaves actor authority `unevaluated`. |
| `actor_authority` | Optional object with exactly the FR-116 `ActorAdmission` members: `actor` and `admission_ref` are strings; `authorized` is an array of strings containing obligation ids. Empty `actor` or `admission_ref`, or an absent `signature_result`, leaves actor authority `unevaluated`; an empty `authorized` array or one that omits the selected obligation is evaluated as `qualification-rejected`. `actor` MUST equal the accepted signature result's mapped signer principal; the input string does not establish identity. A key fingerprint is never an actor. |

The object and each nested input object have no other members. `record` and
`protocol` are the only required root members so absent host observations
remain qualification input gaps rather than invented defaults. Present
`ci_association` and `actor_authority` objects have exactly the members above;
`authorized` may be empty but may not contain empty or duplicate obligation
ids. A non-string where a string is required, a non-array `authorized`, a
non-string array item, a missing required nested member, or an unknown member
is malformed input and is refused; an empty required `protocol` or `record` is
also refused. Repeated option values refer to distinct
canonical record paths and carry the same valid `selected_head` when that
value is supplied. Empty `selected_head` or `checkout_sha` values are validly
typed but unavailable observations, not malformed JSON.

An item applies only to the canonical record named by `record`. The entry,
obligation and declared method are taken from that selected record, its stored
`traceIds` and the selected catalog; the observation file cannot choose or
rewrite them. The host supplies the actual selected source HEAD as
`selected_head`; this value is a read-time input, not persisted provenance. An
observation file is a one-shot input for one read invocation, not a retained
run history or a second evidence store.

The report is not supplied through this host-observation input and its source
bytes are not retained. The selected run's allowlisted report projection is
read from the exact FR-030 companion path
`analysis-reports/<SUITE-N>/<timestamp>.json`, derived from the selected run
path `runs/<SUITE-N>/<timestamp>.json`. The projection and run form one
immutable retained pair. The writer validates the source report through FR-151
before constructing the projection; the reader validates the projection
protocol and its members. The projection has no symbol, request digest or
other run-entry identity. For `agreement`, `inconclusive` and `unavailable`,
the reader requires exactly one run entry and verifies that its outcome is the
one mapped from the disposition and agreed status; for `disagreement`, it
requires a zero-entry run. This one-entry rule prevents a result from matching
one entry among several. The host's FR-036 signature-verification result MUST
identify the verified signed commit C for the exact pair. The host verifies
that the signature-verification result was produced for C's exact commit
object identity; matching tree contents or an unrelated later result is not
enough. The host verifies that C contains both selected files byte-for-byte
and that, relative to C's
first parent (or the empty tree for a root commit), neither canonical path
existed and C adds both paths in that commit. A commit adding only one member,
or a later commit whose tree merely retains the pair, does not establish the
pair's actor authority. The host verifies that the bytes read for both files
are the bytes in C. A missing, unpaired, invalid, changed or run-mismatched projection
cannot support positive qualification and is reported as `unevaluated` or
`record-conflict` under the applicable FR-116 premise. No commit id or content
digest is written into either file.

### What counts

Coverage credit for an obligation backed by an analysis result is the FR-078 `qualified-record` status and nothing
else. A binding in the store, a passing entry and a healthy `quoin evidence audit` result each fall short of it, as
FR-116 states for a healthy audit. Credit therefore needs, in this order:

1. an entry whose obligation the recorder stated with `--discharges` (FR-152, quoin);
2. a passing entry whose binding survives the FR-032 audit of the latest run (quoin);
3. a positive FR-116 decision for the exact selected target and its declared method, from the planned qualifier and
   `quoin-quire` bridge (quoin; planned, not built);
4. FR-078 reconciliation of that decision as a `record_binders` claim (quire-rs; built).

### What each disposition counts as

| Report | Entry (FR-151) | Binding (FR-152) | FR-078 category | Counted |
| --- | --- | --- | --- | --- |
| `agreement`, `satisfied` | `pass` | created when the recorder stated the obligation | qualified claim, or the `unevaluated` / `qualification-rejected` cause of the premise that fails | Only as a qualified claim |
| `agreement`, `refuted` | `fail` | none | `failed` | No |
| `inconclusive` | `error` | none | `errored` | No |
| `unavailable` | `error` | none | `errored` | No |
| `disagreement` | none; `unrepresented` result, run with zero entries | none | `absent` | No |

`inconclusive` and `unavailable` are the same category because the entry does not tell them apart (FR-151); the
retained report does. `disagreement` is `absent` because the zero-entry run holds no result for the obligation; the
`unrepresented` result is printed and not stored (FR-151), so the stored evidence cannot say that two engines
disagreed.

### Qualification of an analysis act

The entry's `evidenceKind` is unset (FR-151), and a stated `evidenceKind` establishes nothing as Quoin FR-116 states,
so neither supplies a qualification premise. Qualification of an analysis entry needs a producer definition that
Quoin FR-116 requires the owner to have approved for this act. The owner has approved it
([FR-184](./FR-184-analysis-act-producer-definition.md)). An analysis-backed obligation is `unevaluated`, and none is
counted, while no FR-093 assessment exists (the bounded-reliance premise) or when the run-package step has no CI
checkout SHA (the performed-revision premise). The credit is consistency evidence only.

### What is decided here and what is not

Decided here, from the text of FR-152, FR-116 and FR-078:

- The binding source is the `--discharges` option of FR-152. A declared target or a plan naming the analysis request
  is not a binding source in this version; adding one would be a change to FR-152.
- Quoin does not judge whether a `satisfied` report analyses the statement of the obligation it is stated for. The
  recorder's assertion is evaluated only by the actor-authority premise of Quoin FR-116.
- A `disagreement` is not distinguishable from an absent result after recording, because the `unrepresented` result is
  printed and not stored (FR-151). A store change under FR-030 would be a separate requirement.

Ruled by the owner, not yet specified in quire-rs:

- Waivers. The FR-116 owner ruled (relayed by the planner 2026-11-13), option A: a waiver is a named person's
  decision with an approval reference, signed through the signed-records feature, reported as its own waiver outcome
  and never as a pass. Strict mode still fails by default; a policy setting must opt in before approved waivers pass.
  It reuses the person-owned disposition pattern of the advisory join (PLAT-1516) and the anomaly release rationale
  (PLAT-1584). The quire-rs FR-078 side still needs its own spec change from the IR and quire-rs owners; until it
  lands nothing is waivable (FR-184 item 9).

## Behavior

- The bridge SHALL ask the FR-116 qualifier about an analysis entry with the obligation's declared method and the
  producer definition the owner has approved for this act.
- `quoin matrix` and the derived retained-evidence view of `quoin assurance` SHALL read each repeated
  `--run-observation FILE` into an FR-116 `RunObservation` value and pass it to the same analysis qualification path.
- The read commands SHALL match each run observation only to the exact canonical run-record path in the selected
  FR-030 store.
- The read commands SHALL read the selected record's allowlisted projection only from its exact FR-030 companion
  path; the observation file SHALL NOT supply, replace or redirect the projection.
- The read commands SHALL validate the projection's `quoin.analysis-report-projection/v1` protocol and closed
  member set before using its contents.
- The projection SHALL contain no symbol, request digest or other run-entry identity.
- For `agreement`, `inconclusive` and `unavailable`, the read commands SHALL require exactly one run entry and
  verify its outcome against the disposition and agreed status before using projection members as qualification
  evidence.
- For `disagreement`, the read commands SHALL require a zero-entry run.
- A `disagreement` projection with a zero-entry run SHALL NOT produce an analysis record claim.
- The host SHALL supply an FR-036 signature-verification result for the exact signed commit C that introduces the
  selected pair.
- The host SHALL verify that the signature-verification result is for C's exact commit object identity; matching
  tree contents or a result for another commit SHALL NOT be treated as a result for C.
- The host SHALL verify that both canonical paths were absent from C's first-parent tree (or the empty tree for a
  root commit) and that C adds both files together.
- The host SHALL reject a signature result for a later commit that merely contains the pair unchanged, or for a
  commit that adds only one member.
- The host SHALL verify that both selected store files are byte-identical to those in the verified commit.
- A signature result for a commit that contains only one member, different bytes, or does not introduce both files
  together relative to its first parent SHALL NOT satisfy actor authority.
- If the run or projection has no valid paired file, the projection fails validation or does not match the run, or either file
  differs from the pair in the verified commit, then the read command SHALL leave the affected result non-qualified
  and report the applicable `record-conflict` or `unevaluated` cause with the missing or mismatched premise.
- The read commands SHALL correlate an observation through the selected record's exact entry, stored obligation
  `traceIds`, selected obligation and declared method.
- The read commands SHALL NOT substitute a suite name, entry symbol, digest or another record's observation for that
  correlation.
- The read commands SHALL use the supplied `selected_head` as FR-116's current selected source HEAD for analysis
  qualification.
- The host SHALL supply the actual selected source HEAD as `selected_head`.
- The read commands SHALL NOT resolve `selected_head` from Git.
- If `selected_head` is absent or is not a full 40- or 64-digit hexadecimal Git object id, then the qualifier SHALL
  report `unevaluated` for the performed-revision premise.
- If supplied `selected_head` values disagree across observation files, then the qualifier SHALL report
  `record-conflict` for affected analysis entries.
- The selected-HEAD observation SHALL apply only to the FR-116 analysis qualification path.
- The selected-HEAD observation SHALL NOT change the legacy audit input assembly or add a commit field to the
  evidence store.
- If `checkout_sha` is absent or is not a full 40- or 64-digit hexadecimal Git object id, then the qualifier SHALL
  report `unevaluated` for the performed-revision premise.
- When a valid `checkout_sha` is supplied, the qualifier SHALL compare it with the selected source HEAD under FR-116.
- If the supplied checkout SHA differs from the selected source HEAD, then the qualifier SHALL report
  `wrong-revision`.
- The bridge SHALL NOT produce `qualified-record` for a wrong-revision result.
- If `ci_association` is absent, then the qualifier SHALL report `unevaluated` for the execution-association premise.
- If `ci_association` names a manual execution, then the qualifier SHALL report `qualification-rejected` for the
  execution-association premise because this act requires CI.
- If a supplied CI association has an empty member, then the qualifier SHALL report `unevaluated` for the
  execution-association premise.
- The read commands SHALL validate the structure and exact record correlation of CI association input only.
- The read commands SHALL NOT contact a CI service or treat matching association strings as external authentication.
- The host SHALL associate each `signature_result` with the commit that added the selected run and its companion
  projection as one evidence pair.
- The read commands SHALL admit actor authority only when the signature result has status `signed_valid`, a mapped
  signer with a non-empty principal, and the corresponding `actor_authority` admits that principal for the selected
  obligation.
- The read commands SHALL use the mapped signer principal as FR-116's actor identity.
- The read commands SHALL NOT use a key fingerprint as the actor identity.
- If the signature result is missing or unverifiable, then the qualifier SHALL report `unevaluated` for actor
  authority.
- If the signature result is verified but invalid, untrusted or unmapped, then the qualifier SHALL report
  `qualification-rejected` for actor authority.
- If host authority excludes the selected obligation, then the qualifier SHALL report `qualification-rejected` for
  actor authority.
- The bridge SHALL NOT produce `qualified-record` when actor authority is `unevaluated` or `qualification-rejected`.
- If no `--run-observation` file selects a selected run record, then the qualifier SHALL retain its
  missing FR-116 premises as `unevaluated`.
- The read commands SHALL NOT borrow observations from another record when no file selects a selected run record.
- If an observation file has malformed JSON or an unsupported protocol, then the read commands SHALL refuse that
  input without a report claiming positive qualification.
- If an observation file has an unknown root, record, association or authority member, then the read commands SHALL refuse that input without a report
  claiming positive qualification.
- If multiple observation files select the same record or a file selects no canonical record in the selected store,
  then the read commands SHALL refuse the inputs without a report claiming positive qualification.
- If an embedded signature result is malformed or non-admitting, then the qualifier SHALL leave actor authority
  unmet under the preceding rules.
- The read commands SHALL validate each JSON member against the types and exact member sets in Inputs before
  constructing FR-116 typed observations; a wrongly typed or structurally incomplete input SHALL cause the read
  commands to refuse that input.
- The read commands SHALL treat an empty `selected_head` or `checkout_sha` as an unavailable performed-revision
  observation, an empty CI-association member as unavailable execution association, and empty `actor` or
  `admission_ref` as unavailable actor authority.
- The read commands SHALL compare `actor_authority.actor` with the mapped signer principal in the accepted signature
  result; a mismatch SHALL be `qualification-rejected` for actor authority.
- An empty `authorized` list or a list omitting the selected obligation SHALL be `qualification-rejected` for actor
  authority; empty or duplicate obligation-id strings SHALL make the input malformed and refused.
- The bridge SHALL count an analysis-backed obligation as covered only by a positive FR-116 decision, reconciled by
  FR-078 as a `record_binders` claim and status `qualified-record`.
- The bridge SHALL correlate a requested target to analysis entries by the `traceIds` stored on the run entries of
  any outcome, so a failing or erroring entry is reported as `failed` or `errored` with its record locators and not
  as an unbound obligation.
- If the bound entry's outcome is `fail`, then the result SHALL be `failed`.
- If the bound entry's outcome is `error`, then the result SHALL be `errored`.
- If the obligation's declared method is not Analysis, then the result SHALL be `qualification-rejected`, naming the
  method and act pairing premise of FR-116 (FR-152).
- If the latest run of a suite holds zero entries, or no entry for the bound symbol, then the result SHALL be
  `absent`, with no invented record locator and no producer outcome.
- The bridge SHALL order the runs of a suite as the store does (FR-030): by the run's `--timestamp`, ties broken by
  the record's file name, whatever order the records are supplied in; the later run is the one that decides.
- If a later run of the same suite fails or errors on the bound symbol of an earlier pass, then the result SHALL be
  `stale`, with the later `failed` or `errored` outcome and its locators visible as FR-078 and FR-116 require, and the
  bridge SHALL NOT count the earlier pass.
- If a later run of the same suite lacks the bound symbol of an earlier pass, then the result SHALL be `absent`.
- If an obligation has more than one bound symbol, then a failing or erroring bound symbol SHALL leave the
  obligation uncounted (FR-032).
- If no FR-093 assessment exists, or the run-package step has no CI checkout SHA, then the result SHALL be
  `unevaluated`, naming the bounded-reliance or the performed-revision premise (FR-184).
- The bridge SHALL NOT count an analysis entry from a binding alone, from `AuditReport.healthy`, or from the
  `unrepresented` result.
- The bridge SHALL NOT judge whether the analysis request is the right analysis for the obligation.
- Trace SHALL expose a counted analysis result as a record claim with its record, run and entry locators, separate
  from source-symbol claims and citations (FR-078); the symbol `<analysisKind>::<requestDigest>` SHALL appear only as
  the retained entry's own identity and SHALL NOT be reported as a source symbol.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-153-CON-1 | The bridge SHALL add no analysis-specific field, status or report member to quire-rs; it uses the FR-078 claim, cause categories and statuses unchanged. | Compatibility | Inspection |
| FR-153-CON-2 | Counting SHALL be a read of the store and supplied host observations: no solver is run, no source differential report is read from outside the record command, and nothing is written (FR-078-AC-20). The read commands consume only the FR-030 allowlisted projection. | Architecture | Test |
| FR-153-CON-3 | Binding, per-suite supersession and the audit's failed-run rule SHALL stay as FR-030 and FR-032 define them; a report recorded into a suite supersedes the earlier report of that suite whatever its `requestDigest`. | Compatibility | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-153-AC-1 | Given a bound `pass` entry, an approved producer definition and every other FR-116 premise admitted, the obligation is `qualified-record` in `record_binders` with the entry's record, run and entry locators, and leaves `unbacked_rows`. | Test |
| FR-153-AC-2 | The same bound `pass` entry with no FR-093 assessment, or with no CI checkout SHA, yields `unevaluated` naming the bounded-reliance or the performed-revision premise, the obligation stays in `unbacked_rows`, and the strict decision rejects. | Test |
| FR-153-AC-3 | An `agreement`/`refuted` entry recorded with `--discharges FR-001-AC-1` yields `failed`, an `inconclusive` entry and an `unavailable` entry each yield `errored`; each carries its record locators, adds no `record_binders` claim and rejects strict mode. | Test |
| FR-153-AC-4 | A `disagreement` report recorded with `--discharges FR-001-AC-1` yields `absent` for that obligation, with no record locator invented and no claim, the same result as an obligation never recorded; the printed `unrepresented` result is the only place the disagreement is visible. | Test |
| FR-153-AC-5 | A `pass` run followed by a later `fail` or `error` on the same symbol leaves the obligation uncounted with the result `stale` and the later `failed` or `errored` outcome visible; a later run of the suite lacking the symbol gives `absent`; supplying the records in the reverse order, and giving the two runs equal timestamps, gives the same result. | Test |
| FR-153-AC-6 | An obligation bound through two suites, one with a passing symbol and one with a failing symbol, is uncounted and reports `failed` with the failing suite's locators; no `record_binders` claim and no `qualified-record` status is produced for the passing suite. | Test |
| FR-153-AC-7 | A second report for a different `requestDigest` recorded into the same suite as a first replaces the first as the suite's latest run, and only the second can count; recorded into a different suite, both are bound and both must pass. | Test |
| FR-153-AC-8 | A bound `pass` entry with a healthy `quoin evidence audit` and no FR-116 decision is not counted: no `qualified-record` status and no `record_binders` claim. | Test |
| FR-153-AC-9 | Forward trace on the counted obligation returns the record claim with its locators and provenance; inverse trace by the record or run locator returns the same claim; a source-symbol query for `<analysisKind>::<requestDigest>` returns no claim, and the id appearing in prose stays a citation. | Test |
| FR-153-AC-10 | A `pass` report recorded without `--discharges` produces no claim for any obligation, even where its symbol or digest text equals an obligation id. | Test |
| FR-153-AC-11 | A passing report stated with `--discharges` for an obligation whose declared method is Test yields `qualification-rejected` naming the method and act pairing premise, with an approved producer definition and every other premise admitted, and is not counted. | Test |
| FR-153-AC-12 | `quoin matrix` and derived `quoin assurance` accept repeatable `--run-observation FILE` inputs using one `quoin.analysis-run-observation/v1` object per canonical run-record path; the inputs supply CI association, performed checkout SHA and actor-authority evidence without adding those fields to FR-030 run records. Both views expose the selected analysis result, FR-078 category, record locators and each unmet premise in their JSON and text output using the additive quoin-assurance output members defined by FR-115 and FR-040; the quire-rs FR-078 wire contract remains unchanged. | Test |
| FR-153-AC-13 | An observation for another record cannot qualify the selected analysis result; the selected entry, obligation and declared method come from the canonical record and catalog; absent observations remain `unevaluated`; and no observation is reused by suite, symbol or request digest. | Test |
| FR-153-AC-14 | Missing or malformed `selected_head` or `checkout_sha` remains `unevaluated` for performed revision; differing valid values yield `wrong-revision`; conflicting supplied `selected_head` values yield `record-conflict`; none produces `qualified-record`. | Test |
| FR-153-AC-15 | An absent CI association or one with an empty member remains `unevaluated` for execution association; a manual association yields `qualification-rejected`; none produces `qualified-record`. | Test |
| FR-153-AC-16 | Only an FR-116 `ActorAdmission` with string `actor` and `admission_ref` and string-array `authorized`, whose actor equals the complete FR-036 signature result's non-empty `signer.principal` for exact commit C with `status` `signed_valid` and `signer.mapping` `mapped`, and whose authorization includes the selected obligation, satisfies actor authority; the host verifies the result targets C's exact commit object identity. C's first-parent tree (or empty tree for a root commit) has neither selected canonical path and C adds both selected files with bytes matching the read pair. A later commit merely containing the pair, a commit adding only one member, a result for another commit with matching tree contents, or a byte mismatch does not satisfy actor authority. Empty actor/reference or missing, malformed or unverifiable signature evidence remains `unevaluated`; empty or out-of-scope authorization, actor/principal mismatch, invalid, untrusted, unmapped or principal-less signatures are `qualification-rejected` naming actor authority. | Test |
| FR-153-AC-17 | Invalid observation-file JSON/protocol, empty required `protocol` or `record`, wrongly typed or incomplete members, unknown members, empty/duplicate obligation ids, duplicate record selectors or selectors outside the selected canonical store are refused without a report that claims positive qualification; a missing or invalid retained projection/run pair also cannot qualify; omitted observation files leave each affected result `unevaluated`. | Test |

## Dependencies

- **Upstream**: [FR-152](./FR-152-analysis-run-entry-link-to-obligation.md) (the stated obligation and binding),
  [FR-151](./FR-151-differential-report-v2-adapter.md), [FR-032](./FR-032-evidence-auditor.md),
  [FR-115](./FR-115-evidence-backed-test-matrix.md) (matrix read path),
  [FR-116](./FR-116-owner-qualified-quire-evidence.md) (qualifier, planned) and
  [FR-093](./FR-093-evidence-producer-trust.md).
- **Cross-repository**: quire-rs [FR-078](ix://agent-ix/quire-rs/FR-078), whose engine side is built
  and unchanged here; engineering-assurance [FR-036](ix://agent-ix/engineering-assurance/FR-036) and FR-037
  define the signature-result schema and mapped signer identity.
