---
id: FR-171
title: "The security analysis skill writes control and scope_exclusion artifacts, not a prose section"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-security/FR-014"
    type: "references"
  - target: "ix://agent-ix/spec-objects-security/FR-016"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-180"
    type: "references"
  - target: "ix://agent-ix/quoin/FR-181"
    type: "references"
---

# FR-171: The security analysis skill writes control and scope_exclusion artifacts, not a prose section

## Description

The `spec-security-analysis` skill writes its result as typed spec-objects-security artifacts: one `control` artifact per
control the component implements, whose `## Mappings` table names clause references, and one `scope_exclusion` artifact per
clause outside the component's boundary. It no longer writes Section 19 "Compliance Standards Traceability" into
`spec/spec.md`.

## Rationale

The prose section cannot be validated, resolved against a clause set or fed to the discharge report. Typed artifacts can.
Nothing is written into `spec/spec.md` because the artifacts are the record.

## Behavior

- The skill's output step SHALL author each control with `quoin write` using the `control` skeleton.
- The skill SHALL fill `## Mappings` with Set and Clause rows (spec-objects-security FR-014).
- The skill SHALL link each control to the FR, NFR or StR it implements, in the control's frontmatter `relationships`.
- The skill SHALL report a clause that applies with no covering requirement as a gap in its reply.
- For a clause outside the component, the skill SHALL author a `scope_exclusion` with `## Set`, `## Clause`, `## Responsible` and `## Rationale` sections (spec-objects-security FR-016). `## Responsible` names the responsible component by the bare id of a bundle artifact.
- When no responsible party is known, the skill SHALL write no exclusion.
- When no responsible party is known, the skill SHALL list the clause as having no responsible party in its reply.
- The skill SHALL write a control's `## Effectiveness` section as `not_assessed` unless a person supplied another value.
- The skill SHALL run `quire validate` over the artifacts it wrote and report failures.
- The skill's template and example SHALL be a `control` and a `scope_exclusion` artifact, replacing the Section 19 template and example.
- The skill SHALL cite NIST SP 800-53 as Rev 5.2.0, the version of the clause set key `nist/sp800-53@5.2.0` ([FR-172](./FR-172-clause-set-sources-and-licence-boundary.md)).
- The change to the skill SHALL correct each NIST SP 800-53 and OWASP ASVS control number that the skill cites and that either is not a control of the cited release of the free source or identifies a control other than the one the citing row describes. A row that carries a shortened, older or reworded title of the control its number identifies is not wrong, and the change does not rename it.
- When an ASVS chapter id or title differs from the cited release, the change to the skill SHALL replace it with the cited release's chapter id and title.
- The PLAT-1426 change to the skill SHALL leave every other standard name in `SKILL.md` and the references of the free sources as it is.
- When a source profile is selected, the skill SHALL run `quoin profile evaluate --out-dir <tmp>` before mapping controls.
- When deciding which clauses apply, the skill SHALL use only the resulting `source-profile-evaluation-v1` document.
- When deciding which clauses apply, the skill SHALL NOT independently recompute clause applicability.
- For a clause whose evaluated outcome is `binding`, the skill SHALL author a `control` or `scope_exclusion` according to the component boundary and the other rules in this requirement.
- For a clause whose evaluated outcome is `unresolved`, the skill SHALL add a SpecReview item naming the missing context dimension.
- For a clause whose evaluated outcome is `unresolved`, the skill SHALL NOT author a `control` or `scope_exclusion` for that clause.
- The skill SHALL NOT map a `not_binding` clause or a clause from a set absent from the selected profile.
- When a selected profile includes a set reported `unavailable`, the skill SHALL report `not mapped, set not installed` for that set.
- When a selected profile includes a set reported `unavailable`, the skill SHALL continue mapping the other evaluated sets.
- When no source profile is selected, the skill SHALL stop before mapping and ask the user to create `.ix/source-profile.yaml`.
- When no source profile is selected, the skill SHALL name only `web-app-baseline`, `us-device-cybersecurity-hipaa` and `eu-device-gdpr-ai-act` as the three available defaults.
- When no source profile is selected, the skill SHALL NOT default to all sources.
- The skill SHALL NOT write or change `.ix/source-profile.yaml` or any `context` value; the repository supplies profile selection and product facts.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-171-AC-1 | `skills/spec-security-analysis/SKILL.md` contains no instruction to write Section 19, and its output step names the `control` and `scope_exclusion` artifacts. | Inspection |
| FR-171-AC-2 | The shipped template and example artifacts each pass `quire validate` with the spec-objects-security module installed. | Test |
| FR-171-AC-3 | `SKILL.md` instructs `not_assessed` as the default `## Effectiveness` value and instructs no exclusion without a responsible party. | Inspection |
| FR-171-AC-4 | A diff of the PLAT-1426 change to the skill against its base shows no changed standard name in `SKILL.md` or the free-source references, other than ASVS chapter ids and titles replaced by those of ASVS 5.0.0. | Inspection |
| FR-171-AC-5 | `SKILL.md` instructs the gap and no-responsible-party reply and the `quire validate` run. | Inspection |
| FR-171-AC-6 | Every NIST SP 800-53 and OWASP ASVS control number cited anywhere under `skills/spec-security-analysis` is a control of Rev 5.2.0 and of ASVS 5.0.0 respectively and identifies the control its row describes, checked against the published catalogues; a number is wrong when the catalogue has no such control, or when the control it identifies is a different control from the one the row's name and description denote, and a shortened, older or reworded title of the same control does not make a number wrong; each wrong number is listed in the PR with its correction, and none remains. | Analysis |
| FR-171-AC-7 | `SKILL.md` names NIST SP 800-53 as Rev 5.2.0 and no other revision string. | Inspection |
| FR-171-AC-8 | Given a selected profile, the skill runs `quoin profile evaluate --out-dir <tmp>` and maps only clauses reported `binding`; a `not_binding` clause and a clause from a set absent from the evaluation document produce no artifact. | Test |
| FR-171-AC-9 | An `unresolved` clause appears in the SpecReview with its missing dimension named and produces no `control` or `scope_exclusion`. | Test |
| FR-171-AC-10 | An `unavailable` selected set is reported `not mapped, set not installed`, while binding clauses from evaluated sets are still mapped. | Test |
| FR-171-AC-11 | With no `.ix/source-profile.yaml`, the skill stops before mapping, emits no control or exclusion, asks for the selection file, and names exactly the three defaults `web-app-baseline`, `us-device-cybersecurity-hipaa` and `eu-device-gdpr-ai-act`. | Test |
| FR-171-AC-12 | Fixture A selects `web-app-baseline` with `baseline=low` and `level=1`; every mapping references only a NIST or OWASP clause reported `binding` by the evaluation document, and a mapping to a `not_binding` clause fails the test. | Test |
| FR-171-AC-13 | Fixture B selects a profile containing only `owasp/asvs@5.0.0`; its output has no NIST mapping, and its mapped source coverage is disjoint from Fixture A where their profile sets differ. | Test |
| FR-171-AC-14 | Before and after a skill run, `.ix/source-profile.yaml` and the evaluated profile context are unchanged; the skill authors neither selection nor product facts. | Test |

## Dependencies

- **Upstream**: spec-objects-security FR-014, FR-016; [FR-180](./FR-180-source-profile-selection.md), [FR-181](./FR-181-source-profile-evaluation.md), [FR-021](./FR-021-default-source-profiles.md)
- **Downstream**: [FR-172](./FR-172-clause-set-sources-and-licence-boundary.md), [FR-173](./FR-173-facts-from-mappings.md)
