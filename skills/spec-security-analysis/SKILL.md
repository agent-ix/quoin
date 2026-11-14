---
name: spec-security-analysis
description: Analyze specifications and implementations to identify applicable security standards and generate compliance traceability documentation.
---

# Security Compliance Analysis

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Analyze a component's specification and implementation to generate security compliance traceability for the selected source profile. Map only clauses the profile evaluation reports as binding.

## Sources

Apply the clause sets named by the selected source profile whose evaluation is available; applicability comes only from that evaluation.

| Standard | Version | Scope |
|----------|---------|-------|
| NIST SP 800-53 | Rev 5.2.0 | Only when its clause set is in the selected profile |
| OWASP ASVS | 5.0.0 | Only when its clause set is in the selected profile |
| EU GDPR | 2016/679 | Only when its clause set is in the selected profile |
| Licence-restricted sources in the effective profile | Exact clause-set key | Controls declared by an installed pack |

## Process

1. **Gather Context**
   - Read `spec/spec.md` (if exists)
   - Scan implementation for security-relevant patterns (auth, data, API, logging)
   - Identify component type and function

2. **Identify Applicable Controls**
   - Use [references/standards-index.md](references/standards-index.md) as secondary context for interpreting selected sources; it does not select sources
   - Select relevant chapters from the indexes for sets in the effective profile:
     - [nist-800-53/index.md](references/nist-800-53/index.md) → Control families
     - [owasp-asvs/index.md](references/owasp-asvs/index.md) → V1-V17 chapters
     - [gdpr/index.md](references/gdpr/index.md) → Chapters II-V
   - Use index decision guides to choose specific chapters to investigate
   - Reference [component-type-mapping.md](references/component-type-mapping.md) for scope guidance
   - Verify identified controls against official FREE sources when the selected profile includes those sets:
     - NIST 800-53: https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final (US Gov - fully open)
     - OWASP ASVS: https://github.com/OWASP/ASVS (CC BY-SA 4.0 - fully open)
     - GDPR: https://eur-lex.europa.eu/eli/reg/2016/679/oj (EU Law - fully open)
   - Local references for the free sources are summaries; use the free sources above for specific control details
   - **Licensed standards come from an installed pack only.** The skill ships no reference table, summary or web source for a licence-restricted source. Its clause ids are read from the clause set of a locally installed pack (`quoin module list` shows `pack=true`) and nowhere else; when no pack is installed the skill does not recall ids, titles or descriptions from memory, prose or the web
   - **A pack is the source's set only by its key.** A pack counts as a source's set only when its exact key appears in the evaluation document; never choose a set by the similarity of its name to the source's. The skill may read an installed pack's clause text locally to understand a binding clause while grouping controls and linking requirements, but it must not change the evaluation's outcome. It never writes any title, text or paraphrase of the clause into an artifact, review or reply; the id and the set key are the whole reference
   - **Evaluate the selected profile before mapping.** Run `quoin profile evaluate --out-dir <tmp>` and use only the resulting `source-profile-evaluation-v1` document to decide applicability. Do not independently recompute applicability. If there is no `.ix/source-profile.yaml`, stop before mapping and ask the user to create it. Name the defaults `web-app-baseline`, `data-platform-baseline`, and `minimal`; do not imply they are currently shipped or selectable unless their profile artifacts are present, and never default to all sources.
   - Do not write or change `.ix/source-profile.yaml` or any `context` value; the repository supplies profile selection and product facts.
   - If evaluation exits nonzero, emits no document, or emits a document that fails schema validation, report the failure and stop without mapping or authoring controls/exclusions.
   - Use only clause sets present in the evaluation document. A clause reported `binding` may be mapped according to the component boundary. For `unresolved`, name the missing context dimension in a SpecReview item and author no control/exclusion for that clause. Do not map `not_binding` clauses or clauses from sets absent from the selected profile.
   - If a selected set is `unavailable`, report `not mapped, set not installed` for that set and continue mapping other evaluated sets.
   - Run `quoin module list` and read installed module manifest `clause_sets` files only to verify clause references; a clause set is keyed `authority/id@version`. The free keys are `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0` when selected and installed.

3. **Map Controls to Requirements**
   - Link each applicable clause to the FR/NFR/StR that covers it
   - Group clauses by the capability that answers them; each group is one control
   - A binding clause that has no covering requirement is a gap
   - A clause is mapped only when its set is installed and the set holds the clause id exactly

4. **Define Scope Boundaries**
   - Identify clauses outside the component's boundary
   - Name the responsible component for each, as the bare id of an artifact in this bundle (for example `COMP-002`)
   - When no responsible party is known, write no exclusion and list the clause as having no responsible party

5. **Generate Output**
   - Write typed artifacts; write no prose compliance section into `spec/spec.md`
   - One `control` artifact per control, authored with `quoin write <repo> --types control` and the skeleton it returns. Template: [assets/control-template.md](assets/control-template.md); example: [assets/control-example.md](assets/control-example.md)
   - One `scope_exclusion` artifact per clause outside the component, authored with `quoin write <repo> --types scope_exclusion`. Template: [assets/scope-exclusion-template.md](assets/scope-exclusion-template.md); example: [assets/scope-exclusion-example.md](assets/scope-exclusion-example.md)
   - Put the files under `spec/security/`, named `CTRL-NNN-<slug>.md` and `SCOPEX-NNN-<slug>.md`
   - Run `quire validate --scope <repo> "spec/security/*.md"` over what you wrote and report every failure

6. **Report**
   - Record each gap as a finding in a `SpecReview` under `reviews/` (see the `spec-review` skill), and repeat it in the reply
   - The reply lists, by set key and clause id: each clause that applies with no covering requirement; each clause outside the component with no responsible party; each source not mapped because its set is not installed

## Control Artifact

- `## Mappings` is a table with the columns Set and Clause, one row per clause: Set is the clause set key (`nist/sp800-53@5.2.0`, `owasp/asvs@5.0.0`, or an installed pack's key), Clause is the clause id
- A mapping never carries a title, quotation, summary or note of a clause; the set key and clause id are the whole reference
- Frontmatter `relationships` carry one `implements` entry per FR, NFR or StR the control answers
- `## Effectiveness` is `` `not_assessed` `` unless a person supplied another value; never write `effective` yourself
- Threats and risks the control addresses are `mitigates` relationships, never mapping rows

## Scope Exclusion Artifact

- Four sections, each one inline-code value: `## Set`, `## Clause`, `## Responsible`, `## Rationale`
- `## Responsible` is the bare id of a bundle artifact, never an `ix://` identity of another repository
- It is a boundary statement, not an approval: it carries no decision, status or approver

## Rules

- Apply only the sets selected in the source profile and the clauses the profile evaluation reports as `binding`
- When a source's clause set is not installed, write no mapping for it and state in the reply "not mapped, set not installed" with the source name
- Never supply the text, title or summary of a licence-restricted clause in any artifact, review or reply; reference such a clause by id only, and only from an installed pack
- Every control links to the FR/NFR/StR it implements through frontmatter `relationships`
- Overlapping clauses from multiple standards reinforce coverage
- Out-of-scope clauses MUST identify the responsible component; with none known, write no exclusion
- Gap identification is required - a clause with no covering requirement is a finding

## References

- Control template: [assets/control-template.md](assets/control-template.md)
- Control example: [assets/control-example.md](assets/control-example.md)
- Scope exclusion template: [assets/scope-exclusion-template.md](assets/scope-exclusion-template.md)
- Scope exclusion example: [assets/scope-exclusion-example.md](assets/scope-exclusion-example.md)
- Standards Index: [references/standards-index.md](references/standards-index.md)
- Component Mapping: [references/component-type-mapping.md](references/component-type-mapping.md)
