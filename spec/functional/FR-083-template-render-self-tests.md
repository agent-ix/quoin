---
id: FR-083
title: "Template render self-tests and module conformance gate"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/US-021"
    type: "implements"
  - target: "ix://agent-ix/quoin/FR-076"
    type: "depends_on"
---

# FR-083: Template render self-tests and module conformance gate

## Description

Quoin's own gate SHALL render every supported variant into an isolated temporary
directory and exercise the rendered repository, and SHALL fail when a surface the
conformance contract requires is missing from the template, so that the template
is verified by instantiation rather than by reading.

## Rationale

A template that has never been instantiated is unverified: every defect it
carries is latent until the first maintainer meets it, and the maintainer meeting
it has no way to tell a template defect from their own. The second failure mode
is slower — the contract moves, the maintained module repositories follow, and
the template silently keeps rendering the previous shape. A conformance contract
that names the required surfaces turns that drift into a failing test in this
repository rather than a discovery in the next migration.

## Inputs

- The template source
- The conformance contract naming the surfaces a rendered repository must carry

## Outputs

- A pass when every variant renders and conforms
- A failure naming the variant and the surface, when one does not

## Behavior

- Quoin's gate SHALL render the `artifact`, `object`, and `mixed` variants into isolated temporary directories.
- Quoin's gate SHALL delete each rendered directory after the checks that read it complete, whether they passed or failed.
- Quoin's gate SHALL assert that no rendered file carries an unresolved template token, a placeholder organization, an absolute path from the rendering machine, a credential, or a private-registry publication default.
- Quoin's gate SHALL assert that every rendered variant carries every surface the conformance contract names.
- Quoin's gate SHALL run `quire validate` over each rendered variant's `spec/` tree.
- Quoin's gate SHALL assert the rendered manifest validates against the vendored module-manifest schema and that every `data_schema` digest matches the file it names.
- Where the schema toolchain is installed, Quoin's gate SHALL run the rendered emit command, asserting that it reproduces the committed schemas byte for byte.
- If the schema toolchain is not installed, then Quoin's gate SHALL fail naming the install command, rather than skipping the emission checks.
- The conformance contract SHALL name each maintained semantic-module repository it is compared against by remote and by one full commit revision, so that two runs of the drift check read the same bytes.
- Quoin's gate SHALL compare the conformance contract with the surfaces the maintained semantic-module repositories carry at those pinned revisions.
- Where a maintained repository is not available at its pinned revision, Quoin's gate SHALL fail naming the repository and the revision, rather than reporting no drift.
- The conformance contract SHALL record, for every surface a maintained repository carries and the contract deliberately does not require, the reason it is exempt.
- If a maintained semantic-module repository carries a surface the conformance contract omits, then Quoin's gate SHALL fail naming that surface.
- Quoin's gate SHALL fail when a rendered variant's Test Matrix carries a `Status` cell outside the archetype's vocabulary.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-083-CON-1 | Quoin's gate SHALL write a rendered variant only under a temporary directory, never inside the repository working tree. | Hygiene | Test (TC-1445) |
| FR-083-CON-2 | The conformance contract SHALL be a declared file, not a list embedded in a test body, so a contract change is reviewable on its own. | Maintainability | Test (TC-1447) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-083-AC-1 | The gate renders all three variants and reports which variant and which surface failed when one does. | Test (TC-1444) |
| FR-083-AC-2 | Every rendered directory is removed after the run, including after a failure. | Test (TC-1445) |
| FR-083-AC-3 | An injected unresolved token, placeholder organization, absolute path, credential, or private-registry default in the template fails the gate naming the file. | Test (TC-1446) |
| FR-083-AC-4 | Removing a surface the conformance contract names fails the gate naming that surface. | Test (TC-1447) |
| FR-083-AC-5 | A surface carried by the maintained module repositories at their pinned revisions, absent from the conformance contract and absent from its exemptions, fails the drift check naming it. | Test (TC-1418) |
| FR-083-AC-8 | A maintained repository that cannot be read at its pinned revision fails the drift check naming the repository and the revision. | Test (TC-1462) |
| FR-083-AC-6 | With the schema toolchain absent, the gate fails naming the install command and reports no skipped emission check. | Test (TC-1448) |
| FR-083-AC-7 | A `Status` cell of `⚠️` injected into a rendered Test Matrix fails the gate. | Test (TC-1440) |
| FR-083-AC-9 | Quoin explicitly declares only its root package as its pnpm workspace, matching the one-importer lock. Unrendered template package files remain generation input, not installable workspace packages. Normal frozen installation, script execution and command execution retain dependency verification; all three rendered variants remain independently exercised outside the workspace. | Test (TC-1597) |

## Dependencies

- **Upstream**: [FR-076](./FR-076-semantic-module-template-variants.md), [FR-080](./FR-080-generated-verification-suite.md), [FR-082](./FR-082-generated-governance-tree.md)
- **Downstream**: [NFR-018](../non-functional/NFR-018-rendered-output-hygiene.md), [NFR-019](../non-functional/NFR-019-deterministic-rendering.md)
