---
name: spec-security-analysis
description: Analyze specifications and implementations to identify applicable security standards and generate compliance traceability documentation.
---

# Security Compliance Analysis

If this plugin is not initialized or an Agent IX command fails, read [the quoin setup guide](https://github.com/agent-ix/quoin/blob/main/setup.md) for its prerequisites and local diagnosis.

Analyze a component's specification and implementation to generate comprehensive security compliance traceability. Apply ALL security standards for maximum coverage - overlapping controls strengthen rather than conflict.

## Mandatory Standards

Apply ALL of these standards to EVERY analysis (scope varies by component function):

| Standard | Version | Scope |
|----------|---------|-------|
| AICPA SOC 2 | 2017 TSC | Trust Services Criteria |
| ISO/IEC 27001 | 2022 | Annex A Controls |
| NIST SP 800-53 | Rev 5.2.0 | Security & Privacy Controls |
| OWASP ASVS | 5.0.0 | Application Security Verification |
| EU GDPR | 2016/679 | Data Protection Articles |

## Process

1. **Gather Context**
   - Read `spec/spec.md` (if exists)
   - Scan implementation for security-relevant patterns (auth, data, API, logging)
   - Identify component type and function

2. **Identify Applicable Controls**
   - Read [references/standards-index.md](references/standards-index.md) for standard selection
   - Select relevant chapters from each standard's index:
     - [soc2/index.md](references/soc2/index.md) → CC1-CC9 categories
     - [iso27001/index.md](references/iso27001/index.md) → A5-A8 themes
     - [nist-800-53/index.md](references/nist-800-53/index.md) → Control families
     - [owasp-asvs/index.md](references/owasp-asvs/index.md) → V1-V17 chapters
     - [gdpr/index.md](references/gdpr/index.md) → Chapters II-V
   - Use index decision guides to choose specific chapters to investigate
   - Reference [component-type-mapping.md](references/component-type-mapping.md) for scope guidance
   - **[REQUIRED]** Verify identified controls against official FREE sources:
     - NIST 800-53: https://csrc.nist.gov/pubs/sp/800/53/r5/upd1/final (US Gov - fully open)
     - OWASP ASVS: https://github.com/OWASP/ASVS (CC BY-SA 4.0 - fully open)
     - GDPR: https://eur-lex.europa.eu/eli/reg/2016/679/oj (EU Law - fully open)
     - SOC 2 TSC: https://en.wikipedia.org/wiki/SOC_2 (reference summary - no login)
     - ISO 27001: https://en.wikipedia.org/wiki/ISO/IEC_27001 (control list - no login)
   - Local references are summaries; use web search for specific control details
   - **Licensed standards are used for id lookup only.** The `soc2/` and `iso27001/` references name controls and describe them briefly, and the two web sources above list them. Use them only to decide which clause ids to look up in an installed pack. Copy no name, title, description or paraphrase of a SOC 2 or ISO/IEC 27001 control into any artifact or reply; the id and the set key are the whole reference
   - **Read the installed clause sets** before writing any mapping. Run `quoin module list`, then read each installed module's manifest `clause_sets` files. A clause set is keyed `authority/id@version`; this skill's free sets are `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0` (both in the `spec-objects-security` module). SOC 2, ISO/IEC 27001 and every other licence-restricted source are served only from a locally installed pack, by clause id. A set is installed only if it appears there.

3. **Map Controls to Requirements**
   - Link each applicable clause to the FR/NFR/StR that covers it
   - Group clauses by the capability that answers them; each group is one control
   - A clause that applies and has no covering requirement is a gap
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

- Apply ALL standards - comprehensive coverage trumps minimal compliance, subject to the installed sets below
- When a source's clause set is not installed, write no mapping for it and state in the reply "not mapped, set not installed" with the source name
- Never supply the text, title or summary of a licence-restricted clause (SOC 2, ISO/IEC 27001) in any artifact or reply; reference such a clause by id only, and only from an installed pack
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
