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
Quoin has no listing command for profiles; it reads the one the selection names.

The selection file is `source-profile.yaml` in the project config root. It holds `profile` (a profile name), and
optionally `context` (dimension name to string value) and `add` (clause-set keys, each `authority`, `id`, `version`).
A repository `context` value replaces the profile's value for the same dimension; a dimension the profile does not
state is added. The profile's `context` is therefore the base of the effective context, and the effective context is
what [FR-181](./FR-181-source-profile-evaluation.md) sends to the engine. The effective set list is the profile's sets
followed by the `add` entries, with duplicates by exact key removed.

A profile does not bind a baseline, level or any other fact about the particular product. The repository supplies it
in `context`; until it does, clauses that need it stay `unresolved` ([FR-181](./FR-181-source-profile-evaluation.md)).

## Inputs

- Installed module roots, each with zero or more `profiles/*.yaml` files.
- The project config root's `source-profile.yaml`.

## Outputs

- The effective profile: name, ordered clause-set keys, and effective context.

## Behavior

- Quoin SHALL read the selected profile from the module `profiles/` directories and from nowhere else.
- When the selection names a profile that no installed module ships, Quoin SHALL refuse it, naming the missing profile.
- When two installed modules ship a profile of the same name, Quoin SHALL refuse it, naming both modules.
- When the selected profile breaks a rule of spec-objects-security FR-020 (format, `name` equal to its file stem, non-empty fields, string context values, no repeated key), Quoin SHALL refuse it, naming the file and the rule.
- When the selection file has an undefined key or no `profile`, Quoin SHALL refuse it.
- When a selection `context` value is not a string, Quoin SHALL refuse it, naming the dimension.
- When the selection file is absent, Quoin SHALL select no profile, evaluate no clause set and report that no profile is selected rather than choosing a default.
- Quoin SHALL NOT infer the profile from repository contents, requirement text or file names.
- Quoin SHALL NOT change a clause-set version; a profile entry names an exact version, and a version no module declares is reported unavailable by FR-181.

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
| FR-180-AC-8 | A profile whose `name` differs from its file stem, or that breaks any other FR-020 rule, is refused with the file and the rule named, even when selected by name. | Test |
| FR-180-AC-9 | A selection `context` value of `2` (an integer) is refused naming the dimension; `"2"` is accepted. | Test |
| FR-180-AC-10 | A `profiles/p.yaml` placed in the repository, outside any module root, is not read: selecting `p` fails as in AC-4. | Test |
| FR-180-AC-11 | Over a repository whose requirement text and file names mention a profile, a run with no selection file selects none. | Test |
| FR-180-AC-12 | A profile entry whose version no installed module declares is kept at that version and is not replaced by a version that is declared. | Test |

## Dependencies

- **Upstream**: [FR-004](./FR-004-resolve-config-root.md), [FR-006](./FR-006-locate-module-roots.md); `agent-ix/spec-objects-security` FR-020 (profile format)
- **Downstream**: [FR-181](./FR-181-source-profile-evaluation.md), [FR-182](./FR-182-local-pack-joins-a-profile.md)
