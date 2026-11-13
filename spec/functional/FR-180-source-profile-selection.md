---
id: FR-180
title: "A repository selects one source profile, and Quoin resolves the sources that apply"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-004"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-006"
    type: "depends_on"
  - target: "ix://agent-ix/spec-objects-security/FR-020"
    type: "depends_on"
---

# FR-180: A repository selects one source profile, and Quoin resolves the sources that apply

## Description

A source profile is a named list of clause sets plus an applicability context, authored as module data
([spec-objects-security FR-020](ix://agent-ix/spec-objects-security/FR-020)). It is a different thing from an
`AssuranceProfile`, which sets assurance rigor; a source profile only says which clause sets a repository answers to
and what is already known about the product.

Profiles live in installed modules, not in the engine and not in a repository. A repository chooses one by name in a
small selection file under the project config root ([FR-004](./FR-004-resolve-config-root.md)). Quoin finds the shipped
profiles by scanning the `profiles/` directory of each module root it already locates ([FR-006](./FR-006-locate-module-roots.md)).

The selection file is `source-profile.yaml` in the project config root. It holds `profile` (a profile name), and
optionally `context` (dimension to value) and `add` (clause-set keys, each `authority`, `id`, `version`). A repository
`context` value replaces the profile's value for the same dimension; a dimension the profile does not state is added.
The result is the effective context, and the effective set list is the profile's sets followed by the `add` entries, with
duplicates by exact key removed.

## Inputs

- Installed module roots, each with zero or more `profiles/*.yaml` files.
- The project config root's `source-profile.yaml`.

## Outputs

- The effective profile: name, ordered clause-set keys, and effective context.

## Behavior

- Quoin SHALL read the selected profile from the module `profiles/` directories and from nowhere else.
- Quoin SHALL refuse a selection naming a profile that no installed module ships, naming the missing profile.
- Quoin SHALL refuse two installed modules that ship a profile of the same name, naming both modules.
- Quoin SHALL refuse a selection file with an undefined key or no `profile`.
- Quoin SHALL use no profile when the selection file is absent, reporting that no profile is selected rather than choosing a default.
- Quoin SHALL NOT infer the profile from repository contents, requirement text or file names.
- Quoin SHALL NOT change a clause-set version; a profile entry names an exact version.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-180-AC-1 | With a module that ships profile `p` and a selection file naming `p`, the effective profile lists `p`'s sets in profile order. | Test |
| FR-180-AC-2 | A repository `context` entry replaces the profile's value for that dimension and adds a dimension the profile lacks; other profile dimensions are unchanged. | Test |
| FR-180-AC-3 | `add` entries follow the profile's sets, and an `add` entry equal to a profile set by exact key appears once. | Test |
| FR-180-AC-4 | A selection naming a profile no installed module ships exits non-zero and names the profile. | Test |
| FR-180-AC-5 | Two installed modules shipping the same profile name exit non-zero and name both modules. | Test |
| FR-180-AC-6 | A selection file with an unknown key, or without `profile`, is refused. | Test |
| FR-180-AC-7 | With no selection file, the result states that no profile is selected and no clause set is evaluated. | Test |

## Dependencies

- **Upstream**: [FR-004](./FR-004-resolve-config-root.md), [FR-006](./FR-006-locate-module-roots.md); `agent-ix/spec-objects-security` FR-020 (profile format)
- **Downstream**: [FR-181](./FR-181-source-profile-evaluation.md), [FR-182](./FR-182-local-pack-joins-a-profile.md)
