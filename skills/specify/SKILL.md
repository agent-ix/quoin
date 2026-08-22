---
name: specify
description: Create or update spec artifacts from a design or change request. Authors requested requirements and, when explicitly needed, installed assurance artifacts as discrete files using quoin catalog templates and Quire validation.
---

# Specify

Use this skill when turning an agreed design, feature, or change request into
spec artifacts. `specify` decides **which artifacts the request needs**, authors
each one **as its own file** using the catalog templates, and validates.

## Scope to the request

Author **only the artifact types the request calls for** — nothing more. Assurance
artifacts are opt-in: a low-impact request that asks for an ordinary requirement does
not receive an unsolicited profile, architecture description, or measurement plan.

- "Add a user story for X" → one US.
- "Add the FR that implements US-003" → one FR (traced to US-003).
- "Add a user story and the requirement that implements it" → one US **and** one FR.
- "Edit FR-002 to …" → edit that one FR file in place.
- "Define the assurance profile for this recovery change" → one installed
  `AssuranceProfile`, linked to the requirements or decisions it governs.
- "Describe the architecture and measurement plan for this change" → one installed
  `ArchitectureDescription` and one installed `MeasurementPlan`.
- "Start a spec for service X" / "backport a spec from this code" → the full
  chain (`spec.md` + US + FR + NFR, and IT where there are external integrations).

When unsure which types a request implies, decide from the request and state what
you are creating. A backport or new feature almost always needs **FR artifacts**,
not just a summary.

## Requirements are files, not table rows

Each requirement is a **discrete artifact file** with `type:` frontmatter. The
Requirements table inside `spec.md` (master-requirements) is an **index/summary**
— it never replaces the artifacts. Author every requested requirement as its own
file under the matching directory:

| Type | `type:` | Location                       |
| ---- | ------- | ------------------------------ |
| StR  | `StR`   | `spec/stakeholder/StR-XXX-*.md`|
| US   | `US`    | `spec/usecase/US-XXX-*.md`     |
| FR   | `FR`    | `spec/functional/FR-XXX-*.md`  |
| NFR  | `NFR`   | `spec/non-functional/NFR-XXX-*.md` |
| IT   | `IT`    | `spec/integration/IT-XXX-*.md` |
| Assurance profile | `AssuranceProfile` | `spec/assurance/AP-XXX-*.md` |
| Architecture description | `ArchitectureDescription` | `spec/assurance/AD-XXX-*.md` |
| Measurement plan | `MeasurementPlan` | `spec/assurance/MP-XXX-*.md` |

> If a request asks for functional requirements and you produced only table rows
> in `spec.md`, the request is **not done** — create the `FR-XXX.md` files.

## Writing good requirements

The templates and Quire enforce *shape*; you supply the *judgment*. Author it well
here so `spec-review` has less to send back. Three principles — full guidance with
examples in [references/writing-good-requirements.md](references/writing-good-requirements.md):

- **Scope: tight beats broad.** Prefer a narrow spec that fully defines its slice over
  a sprawling, vague one. Treat `spec.md` §2.2 Out of Scope as first-class — decide
  your non-goals deliberately; an empty Out-of-Scope is a smell.
- **User stories: INVEST + no solution-prescribing.** Describe the user need with a
  real "so that …"; let the FRs pick the mechanism.
- **Acceptance criteria: cover the unhappy paths, be concrete.** Author happy/error/edge
  and negative cases up front, in measurable terms ("returns HTTP 422 within 200ms",
  not "handles errors gracefully").

## Process

Run only the steps the request needs; stop after the requested artifact set.

1. Identify the repository that owns the spec files, and which artifact types the
   request implies (any subset of StR, US, FR, NFR, IT and installed assurance
   artifact types). When the request explicitly involves assurance scope, architecture
   decisions, or reproducible measures, read
   [references/assurance-artifacts.md](references/assurance-artifacts.md). Do not read or
   apply that reference for an ordinary request with no assurance need.
2. Run `quoin write <repo_dir> --types <type[,type...]>` **once** for the full
   set. Use the returned skeletons, schemas, module roots, and examples as the
   authoring contract — the per-type rules live in the template, not your memory.
   The pack also reports the repo's `Org:`; use that value for the `org:` field
   and for the `ix://<org>/<repo>/…` targets you write. If it reports
   `Org: unresolved`, ask the user which organization owns the repo — never
   invent one, never leave a literal `<your-org>` in a file, and never copy the
   org out of a skeleton's example values.
3. Author each artifact as its own file (table above), in this logical order,
   skipping any type not requested:
   - `spec.md` — **REQUIRED when starting a new spec**: the
     `type: master-requirements` root/index at `spec/spec.md`, authored from
     `assets/spec-template.md` (or `assets/app-spec-template.md` for an
     application); keep scope tight and fill Out of Scope deliberately
     (see [references/writing-good-requirements.md](references/writing-good-requirements.md)).
     Then → StR → US (INVEST, no solution-prescribing — same reference) →
     **derive FR from US** (see [references/us-to-fr.md](references/us-to-fr.md))
     → FR (cover unhappy paths, measurable ACs — same reference) → NFR → IT
     (see [references/integration-tests.md](references/integration-tests.md))
     → requested assurance artifacts.
4. Keep traceability in frontmatter `relationships:` (e.g. a US `traces_to` its
   FRs; an FR/NFR/IT links back to the US/FR it serves). Link an assurance artifact
   to the exact requirements or decisions it qualifies using only relationship verbs
   admitted by its fetched schema; do not invent an edge.
5. Run `quire validate <glob> [glob2 ...]` over the changed spec scope and fix
   every error before continuing.
6. **Open in the review cockpit.** After validation passes, run
   `filament open <primary-artifact>` to open the authored artifact in the running
   filament-ide cockpit (it fires the `filament-ide://` deep link and prints a
   clickable link as a fallback). Use the main artifact you created — the
   new/edited `FR-XXX.md`, or `spec/spec.md` when starting a spec. Skip silently if
   the `filament` command is not installed.
7. **Stop after the requested artifacts.** Then ask the user whether to build the test
   matrix (`spec-matrix`) and run review (`spec-review`) — unless they already
   requested them. Only run those on confirmation.

## Functional requirements

FRs are normative (`SHALL` / `SHALL NOT`), atomic, and independently verifiable,
each with an Acceptance Criteria table (the `fr.md` skeleton enforces the shape).

- Derive FRs from the driving user story — see
  [references/us-to-fr.md](references/us-to-fr.md).
- Make the ACs count: cover happy/error/edge and negative cases in concrete,
  measurable terms — see
  [references/writing-good-requirements.md](references/writing-good-requirements.md).
- A behavioral FR has no `object:` field. A **domain-typed** FR carries
  `object: <type>` (e.g. `entity`, `event`, `api_endpoint`, `process`,
  `configuration`) and uses that type's structural template. Look up the
  type→template map and relationship verbs in
  `spec-object-review`'s `references/object-type-guide.md`, and inspect a type
  with `quoin catalog show <type>`.

## Application specs

When `component_type: application`, initialize `spec.md` from
`assets/app-spec-template.md`. Application specs own **only cross-service
concerns** — architecture rationale (ADRs), cross-service sequence diagrams,
system-level NFRs, the external consumer contract, scope boundaries, security
model. Per-service detail (endpoint contracts, DB schemas, per-service NFRs)
stays in the individual service specs. Otherwise use `assets/spec-template.md`.

## Authoring rules

- Let frontmatter identify each file. The discriminator is `type` (e.g.
  `type: FR`, `type: master-requirements`) — it names the archetype.
- Use catalog skeletons and schemas rather than memory.
- If an assurance type is not installed, report that boundary; do not invent its
  frontmatter or silently substitute a generic document.
- Keep workflow definitions out of artifact/object type vocabularies.
- Validate every created or edited artifact with Quire before finishing.

## Reserved OKF Files

`spec-artifacts-iso` ships two reserved archetypes for the OKF folder structure;
author them like any other type:

- `index.md` — `type: index`, body is a `## Contents` link list pointing at the
  artifacts in the folder.
- `log.md` — `type: log`, body is a `## History` of changes to the folder's specs.

## Common Spec Layout

```text
spec/
├── spec.md
├── stakeholder/      # StR-XXX
├── usecase/          # US-XXX
├── functional/       # FR-XXX
├── non-functional/   # NFR-XXX
├── integration/      # IT-XXX
├── assurance/        # requested AP/AD/MP artifacts (when their module is installed)
├── matrix/
└── analysis/
```
