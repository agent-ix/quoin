---
type: tutorial
description: Use Quoin's baseline records to prepare a person-led software risk and release review.
---

# Prepare a software risk and release review

This tutorial follows a software change from authored risk records to an
exported dossier using Quoin's baseline capabilities.

## Before you start

Install Quoin and Quire, initialize a project repository, and identify the
person who owns risk decisions and release approval. Keep one terminal at the
project root for the commands below.

## Intended use

Quoin helps a team author structured requirements and risk records, relate them
to verification evidence, and prepare reports for human review. It provides
repeatable views of records and declared policies. A qualified person decides
the product's intended purpose, risk acceptability, safety class, required
activities, and release approval.

Quoin does not assign safety classes, calculate risk scores, or decide whether
a risk is acceptable. A report records the inputs and rules supplied to it; a
passing command does not establish that a product or process meets a regulation
or standard. The responsible organization defines the review process, selects
and validates the tools for its use, and retains the rationale and approvals.

The baseline path is useful when a team needs traceable records across
requirements, risk analysis, evidence, anomalies, approval, release history,
and a generated dossier. It does not replace product-level validation,
clinical or usability assessment, independent review, or the organization's
quality system.

## Step 1: Prepare the module store

Choose one stable user configuration root. The native Quoin CLI uses
`IX_HOME` or the global `--config-root` option for this root:

```sh
export IX_HOME="$HOME/.ix"
quoin module ensure-defaults
quoin module doctor
quoin catalog validate
```

If `module doctor` reports a default module below the CLI's pin, update that
module explicitly:

```sh
quoin module remove <module-name>
quoin module ensure-defaults
```

`ensure-defaults` skips a module already installed by name. It does not upgrade
an older installed reference. The registry records paths to installed module
directories, so keep the selected configuration root at a stable path after
installing modules. `IX_CONFIG_ROOT` is not the native CLI's environment
variable; use `IX_HOME` or `--config-root`.

## Step 2: Author the requirements and risk chain

Use `quoin write` to discover the available artifact and object templates, then
author the product-specific records in the project. A risk chain should connect
the relevant hazardous situation and harm to the hazard, controls, verification
evidence, and a person's residual-risk decision. Record the rationale and
identity of the person who makes each decision.

Validate the records with Quire and inspect relationship findings before
continuing. A clean relationship report means the declared links satisfy the
loaded rules; it does not judge the product's risk rationale.

## Step 3: Review the matrix and evidence

Use `quire matrix --strict` to check the static requirement and design
relationships. Use `quoin matrix` and `quoin evidence audit` to inspect retained
verification evidence. The Quire strict matrix can pass without any test-run
evidence, so review the evidence axis separately.

When applying class-specific evidence floors, supply a person-authored policy
with `--class-policy`. The YAML or JSON policy has a `class_field`, a `classes`
mapping, and a rationale for each configured class. It may declare required
methods, a mutation floor, independence dimensions, activities, and `gate`.
For example:

```yaml
class_field: software_safety_class
gate: false
classes:
  class-2:
    required_methods: [unit-testing]
    rationale: Reviewed project evidence policy
```

Class values and floors come from the policy and authored records. Quoin does
not infer or recommend them. A class floor changes the audit exit only when the
policy sets `gate: true` and a class finding remains.

## Step 4: Review the SOUP inventory

SOUP means software of unknown provenance.

Use `quoin soup promote <sbom.json>` to prepare records from a software bill of
materials (SBOM), then review those records against supplier information, versions,
licenses, known anomalies, and support status. Run `quoin soup report` with the
applicable binding and evaluation date. The report states missing or unknown
values; a person determines whether the inventory and dispositions are
adequate.

## Step 5: Gate anomalies

Run `quoin anomaly check` and inspect `quoin anomaly report`. Use
`quoin anomaly report --fail-on unassessed` when an unassessed anomaly must fail
the gate. The report carries authored assessment state and rationale. A named
reviewer decides whether each anomaly is acceptable for the proposed change and
records that decision through the applicable approval process.

## Step 6: Sign the change approval

Prepare the change-assurance record, review its complete content, and create a
signed approval tag with the configured signing key. Verify the tag against the
organization's trusted signer configuration before treating it as approval.
The signature proves what the configured key signed; the organization remains
responsible for signer identity, authority, and interpretation.

## Step 7: Prepare release history and the release record

Create and verify the signed release tag using the organization's trusted
signer configuration. Generate the release snapshot from the reviewed source
records, and obtain the Engineering Assurance release-history and release
verification results required by the release record.

Engineering Assurance request bodies are a separate interface. Its help output
does not expose a complete schema for every request. Check its documentation
before submitting each request; Quoin does not define those payloads.

## Step 8: Generate and check the dossier

Generate the selected dossier documents from the same reviewed records,
snapshot, and verification results. Repeat `--module <PATH>` for each module
root when you need a fixed module set; without explicit roots, dossier sources
use the project or ambient module selection. Review every document's `Gaps`
section, then run `quoin dossier check` after committing the generated files.

Generation status describes whether the selected inputs supplied the document's
required fields. It is not a submission-readiness decision.

## Step 9: Export the committed dossier

Commit the current dossier documents, then run `quoin dossier export` with the
desired format and output path. Export checks that the files match regenerated
content at the source commit and delegates PDF or DOCX rendering to Quire.
Review the exported artifact and its source commit before approval or use.

For `quoin evidence export-test-result`, `--subject <FILE>` names a JSON array
of in-toto resource descriptors. Every descriptor needs a 64-character
lowercase SHA-256 digest and a non-empty `annotations.revision`; all revisions
must match the verified execution provenance. Quoin preserves the caller's
subject and does not compute its digest or resolve its revision.

## Known limitations

- Quoin is not a compliance oracle, and this baseline does not include a
  tool-validation package.
- The human organization supplies intended purpose, class, risk decisions,
  policies, reviewer authority, and acceptance rationale.
- Static matrix success does not establish that verification was run or passed.
- The default modules do not include regulatory clause packs. A pack-specific
  assessment requires the applicable pack and separately reviewed mappings.
- The records-of-processing dossier document is omitted from an unfiltered
  export without `--release`; an explicit selection also requires that flag.
- A generated dossier can contain gaps and customer-owned sections. Review each
  document and its source evidence before using it in a submission.
- `quoin dossier export --release` follows Quire's source-commit and tag rules;
  the export must satisfy those rules for the selected dossier directory.
- The evidence recorder transcribes supplied test output; it does not execute
  tests. Check reported bindings and unmatched entries before relying on the
  recorded evidence.
- The in-toto Test Result export is unsigned. Quoin preserves the caller's
  subject; Engineering Assurance authenticates execution provenance.
- Module registry entries include paths under the selected configuration root.
  Moving that root can leave registry paths unusable.
- The functional-requirement `Dependencies` skeleton and manifest contract are maintained by
  `spec-artifacts-iso`; consult that module's current manifest when authoring
  those sections.

## Expected output

You should have reviewed risk and evidence reports, a signed change approval, a
release snapshot and verification result, current dossier documents, and an
exported PDF or DOCX. Each artifact remains subject to the human review and
acceptance process described above.

## Next steps

- Read [setup](../setup.md) to diagnose local CLI configuration.
- Read [assurance workflows](assurance-workflows.md) for evidence and approval
  records.
