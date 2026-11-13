---
id: FR-172
title: "Mapped standards are served as clause sets: publishable sets here, licensed sets from local packs by clause id"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-171"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-137"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-138"
    type: "depends_on"
---

# FR-172: Mapped standards are served as clause sets: publishable sets here, licensed sets from local packs by clause id

## Description

Every standard the skill maps is a clause set. The skill writes a mapping only to a clause that exists in an
installed clause set. Four sources have a stated home:

- NIST SP 800-53 Rev 5 and OWASP ASVS 5.0 are free sources. They are published as clause sets in the `spec-objects-security`
  module, keyed `nist/sp800-53@r5` and `owasp/asvs@5.0.0`, so a public repository can name them and `check_publishable` reports
  nothing for either. Authoring the sets is PLAT-1521 (NIST) and PLAT-1522 (ASVS); this requirement fixes their keys and home, and
  those tickets fix their classification and applicability dimensions and their rights metadata. Each set carries rights metadata
  that states its source licence truthfully, and carries text only as that licence allows. ASVS is share-alike licensed, so its
  terms are recorded in its rights metadata and are never dropped by copying.
- GDPR and HIPAA clause sets come from the EU and US packs (PLAT-1627 and PLAT-1612). This requirement owns neither.
- Every other source the skill maps (SOC 2, ISO/IEC 27001) is licence-restricted. It is served only from a locally installed
  pack (FR-137), by clause id. The skill SHALL NOT supply its text, and a pack is never in the default module set (FR-138).
- FDA cybersecurity clause sets come from PLAT-1593 and are not owned here.

## Behavior

- When the skill is about to write a mapping, it SHALL read the installed clause sets (`quoin module list` and the clause set of the module).
- The skill SHALL write the mapping only when the set key and clause id are present.
- When a source's clause set is not installed, the skill SHALL write no mapping for that source.
- When a source's clause set is not installed, the skill SHALL state in its reply that the source was not mapped because its set is not installed.
- The skill SHALL write no standard text, title or summary into any artifact or reply it produces for a licence-restricted source.
- The two publishable clause sets SHALL declare rights metadata that matches their source licence.
- `check_publishable` SHALL return an empty list for the module that ships the two publishable clause sets.
- The skill's existing local reference summaries are unchanged by this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-172-AC-1 | `check_publishable` returns no restricted set for the `nist/sp800-53@r5` and `owasp/asvs@5.0.0` clause sets. | Test |
| FR-172-AC-2 | Each of the two publishable sets declares rights metadata naming its source licence, and carries no text beyond what that licence allows. | Inspection |
| FR-172-AC-3 | Against a synthetic licensed pack `example.invalid/std@1` installed in a temp `IX_HOME`, the skill instructions yield mappings only for clause ids present in the pack, and a source with no installed set yields no mapping and a not-mapped statement. | Demonstration |
| FR-172-AC-4 | No artifact the skill writes for the synthetic pack contains the pack's clause-text sentinel. | Test |
| FR-172-AC-5 | `SKILL.md` instructs the installed-set check, the not-mapped statement and the no-text rule. | Inspection |

## Dependencies

- **Upstream**: [FR-171](./FR-171-security-analysis-emits-typed-controls.md), [FR-137](./FR-137-detect-and-place-licensed-packs.md), [FR-138](./FR-138-packs-never-come-from-hosted-sources.md)
- **Downstream**: [FR-173](./FR-173-facts-from-mappings.md); PLAT-1593, PLAT-1612, PLAT-1627 supply sets this one does not
