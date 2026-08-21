---
id: FR-045
title: "Bidirectional trace and evidence-graph analyses"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/quoin/FR-030"
    type: "requires"
  - target: "ix://agent-ix/quoin/FR-040"
    type: "requires"
---

# FR-045: Bidirectional trace and evidence-graph analyses

## Description

A trace link is useful in both directions. Downward, it explains what a claim or requirement causes:
derived requirements, verification obligations, and the suites that discharge them. Upward, it
explains why an implementation or obligation exists and which claims depend on it. `quoin graph`
turns the authored document relationships and the existing evidence bindings into three read-only
views:

- **fan-out** counts the distinct obligations discharged by each suite;
- **change-impact** walks from a changed document, obligation, or suite to the artifacts and evidence
  that require review; and
- **churn** ranks obligations by distinct re-affirmation events.

The views expose facts and closure, not an unnamed policy. A high fan-out count is not automatically a
failure, and the engine defines no maximum churn-event count. Callers may use the data in a declared
profile, but the generic graph layer does not smuggle that profile into the measurement.

### Direction is typed, not guessed

Documents author their relationships as source → target: a derived requirement `derives_from` its
parent and an integration test `verifies` a requirement. For relationships whose dependency direction
is known, the graph normalizes the edge into its impact direction. `covers`, `depends_on`,
`derives_from`, `extends`, `implements`, `mitigates`, `refines`, `requires`, `satisfies`, `traces_to`,
and `verifies` flow target → source. `constrains`, `satisfied_by`, and `specifies` flow source → target.
This distinction is load-bearing: an NFR `constrains` an FR in the opposite authored direction from an
FR that `extends` an interface.

Other verbs are not assumed to mean the same thing. For example, the impact direction of `publishes`
cannot be recovered merely from the two ids: the publisher, the event schema, or both may be the
change source. Such an edge is an explicit `unsupported-relationship` limitation until its semantics
are declared.

A full `ix://org/component/id` target resolves to a local id only when the bundle's master-requirements
declares the same organization and component. An external `FR-053` never aliases a local `FR-053`
merely because their last path segments match; cross-repository resolution remains an explicit
limitation until the external graph is supplied.

### Change impact is not an undirected connected component

Walking every edge in both directions would eventually label an entire well-connected bundle as
affected. The view instead distinguishes four results:

- changed documents and their downstream dependents;
- upstream claims and prerequisites, which are review context rather than automatically suspect;
- obligations owned by the changed/downstream documents, and the suites bound to them; and
- other obligations sharing those suites, labelled **shared-suite exposure**, not silently promoted to
  suspect.

A directly changed suite makes the obligations it discharges suspect but does not declare every other
suite for those obligations changed. A directly changed obligation affects that obligation, not every
sibling criterion in the same requirement.

### Churn counts judgements, not copies

An affirmation is an obligation-level judgement. The store copies the event to every selected
`(obligation, suite)` binding so each binding retains its history. Churn therefore deduplicates events
by obligation, committer, commit, and note before counting them; otherwise adding a second suite would
double the apparent wording instability without another review having occurred.

### Incomplete stays incomplete

Every result carries `complete` and `limitations`. Duplicate document ids, unreadable frontmatter,
unresolved relationship targets, unsupported relationship verbs, obligations without an owning
document, and bindings for obligations no longer derived all make the graph incomplete. A
change-impact query for an unknown id also makes that result incomplete and names the id in `unknown`.
These are not command failures: the partial facts are useful, but the output cannot be represented as
a complete closure.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-045-AC-1 | Child-authored dependency and elaboration relationships are normalized to deterministic upstream → downstream document edges. | Test (TC-291) |
| FR-045-AC-2 | Every unreadable document, duplicate id, unresolved or unsupported relationship, orphan obligation, and orphan binding is named as a limitation and makes the graph incomplete. | Test (TC-292) |
| FR-045-AC-3 | Fan-out reports each suite's distinct obligations and count, ordered by count then id, without assigning a pass/fail threshold. | Test (TC-293) |
| FR-045-AC-4 | A changed document reaches its downstream documents, their obligations and suites, its upstream context, and separately labelled obligations sharing an affected suite. | Test (TC-294) |
| FR-045-AC-5 | A changed suite makes its own bound obligations suspect without declaring sibling suites changed. | Test (TC-295) |
| FR-045-AC-6 | An unknown changed id is returned in `unknown`, makes the result incomplete, and never renders an empty closure as a complete answer. | Test (TC-296) |
| FR-045-AC-7 | Churn counts distinct obligation-level affirmation events rather than copies of an event across suite bindings. | Test (TC-297) |
| FR-045-AC-8 | JSON and Markdown are deterministic, and human output labels incomplete graphs before listing their limitations. | Test (TC-298) |
| FR-045-AC-9 | `quoin graph --view change-impact --changed <id>` joins live obligations, authored relationships, and stored bindings through the shipped command surface. | Test (TC-299) |

## Constraints

| ID | Constraint | Type | Validation |
|----|-----------|------|------------|
| FR-045-CON-1 | The views SHALL run no verification and write nothing; they read the spec, current obligations, and evidence bindings. | Design | Inspection |
| FR-045-CON-2 | The graph layer SHALL expose measurements and closure without a universal fan-out or churn threshold. | Design | Test (TC-293) |
| FR-045-CON-3 | The analyzer SHALL NOT infer impact direction for undeclared relationship semantics. | Design | Test (TC-292) |

## Dependencies

- **Upstream**: [FR-030](./FR-030-evidence-store.md) (bindings and affirmation history), [FR-040](./FR-040-assurance-case-view.md) (the read-only graph-view precedent), [FR-037](./FR-037-declared-vocabulary-completeness.md) (the shared bundle reader)
- **Downstream**: assurance profiles may turn these facts into declared review or gating policy; no policy lives in this generic view
