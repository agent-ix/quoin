---
id: FR-180
title: "The security analysis skill writes control and scope_exclusion artifacts, not a prose section"
type: FR
relationships:
  - target: "ix://agent-ix/spec-objects-security/FR-020"
    type: "references"
  - target: "ix://agent-ix/spec-objects-security/FR-023"
    type: "references"
---

# FR-180: The security analysis skill writes control and scope_exclusion artifacts, not a prose section

## Description

The `spec-security-analysis` skill SHALL write its result as typed spec-objects-security artifacts: one `control`
artifact per control the component implements, whose `## Mappings` table names clause references, and one
`scope_exclusion` artifact per clause outside the component's boundary. It SHALL NOT write Section 19
"Compliance Standards Traceability" into `spec/spec.md`.

## Rationale

The prose section cannot be validated, resolved against a clause set or fed to the discharge report. Typed
artifacts can. Nothing is written into `spec/spec.md` because the artifacts are the record.

## Behavior

- The skill's output step SHALL author each control with `quoin write` using the `control` skeleton and SHALL fill `## Mappings` with `Set | Clause` rows (spec-objects-security FR-020).
- The skill SHALL link each control to the FR, NFR or StR it implements, in the control's frontmatter `relationships`, and SHALL report a clause that applies with no covering requirement as a gap in its reply.
- For a clause outside the component, the skill SHALL author a `scope_exclusion` with an `owner` naming the responsible component by `ix://` id and a rationale (spec-objects-security FR-023); when no owner is known it SHALL write no exclusion and SHALL list the clause as unowned in its reply.
- The skill SHALL run `quire validate` over the artifacts it wrote and report failures.
- The skill's template and example SHALL be a `control` and a `scope_exclusion` artifact, replacing the Section 19 template and example.
- The skill SHALL write a control's `effectiveness` row as `not_assessed` unless a person supplied another value.
- The change to the skill SHALL correct control numbers that are wrong against the free sources and SHALL leave every standard name in `SKILL.md` and the references as it is.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-180-AC-1 | `skills/spec-security-analysis/SKILL.md` contains no instruction to write Section 19, and its output step names the `control` and `scope_exclusion` artifacts. | Inspection |
| FR-180-AC-2 | The shipped template and example artifacts each pass `quire validate` with the spec-objects-security module installed. | Test |
| FR-180-AC-3 | `SKILL.md` instructs `not_assessed` as the default `effectiveness` and instructs no exclusion without an owner. | Inspection |
| FR-180-AC-4 | A diff of the skill against its base shows no changed standard name outside a corrected control number. | Inspection |
| FR-180-AC-5 | `SKILL.md` instructs the gap and unowned-clause reply and the `quire validate` run. | Inspection |

## Dependencies

- **Upstream**: spec-objects-security FR-020, FR-023
- **Downstream**: [FR-181](./FR-181-clause-set-sources-and-licence-boundary.md), [FR-182](./FR-182-facts-from-mappings.md)
