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
  - target: "ix://agent-ix/spec-objects-security/FR-017"
    type: "references"
---

# FR-172: Mapped standards are served as clause sets: publishable sets here, licensed sets from local packs by clause id

## Description

Every standard the skill maps is a clause set. The skill writes a mapping only to a clause that exists in an installed clause
set. Four sources have a stated home:

- NIST SP 800-53 Rev 5.2.0 and OWASP ASVS 5.0.0 are free sources. They are clause sets of the `spec-objects-security` module
  (its FR-017), keyed `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0`. The keys carry the exact release because a clause set is
  compared and stored by exact version. Authoring them is PLAT-1521 (NIST) and PLAT-1522 (ASVS), which also fix their
  classification and applicability dimensions.
- GDPR and HIPAA clause sets come from the EU and US packs (PLAT-1627 and PLAT-1612). This requirement owns neither.
- FDA cybersecurity clause sets come from PLAT-1593 and are not owned here.
- Every other licence-restricted source the skill maps is served only from a locally installed
  pack (FR-137), by clause id, and a pack is never in the default module set (FR-138).

## Behavior

- When the skill is about to write a mapping, it SHALL read the installed clause sets (`quoin module list` and the clause set of the module).
- The skill SHALL write the mapping only when the set key and clause id are present.
- When a source's clause set is not installed, the skill SHALL write no mapping for that source.
- When a source's clause set is not installed, the skill SHALL state in its reply that the source was not mapped because its set is not installed.
- The skill SHALL NOT supply the text, title or summary of a licence-restricted clause in any artifact or reply it produces.
- The skill SHALL NOT ship a reference table, summary or index of the controls of a licence-restricted source.
- The skill SHALL NOT instruct a step that reads such a table, summary or index.
- When a licensed pack is installed, the skill SHALL take the clause ids of its source only from the pack's clause set and write them by reference.
- The skill MAY read the clause text of a locally installed pack to judge which clause applies.
- The skill SHALL NOT write any title, text or paraphrase of a restricted clause into an artifact, review or reply; mappings carry clause ids and set keys only.
- A locally installed pack SHALL count as a source's set only when its set key (`authority/id@version`) is the one the user's `source-profile.yaml` lists in `add` ([FR-182](./FR-182-local-pack-joins-a-profile.md)).
- The skill SHALL NOT choose a source's set by the similarity of its name to the source's.
- The NIST and ASVS clause sets SHALL declare a `rights.text` value of `none`, `original` or `explicitly_cleared`, chosen to match their source licence, and never `licensed`, so that quire-rs FR-083 treats them as publishable.
- `quire publish-check` SHALL accept the module that ships the NIST and ASVS clause sets.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-172-AC-1 | `quire publish-check --module <spec-objects-security>` exits 0 once the NIST and ASVS sets are present; the sets are authored under PLAT-1521 and PLAT-1522 and this criterion is run by those tickets' tests. | Test |
| FR-172-AC-2 | The `rights.text` value of each of the two sets is `none`, `original` or `explicitly_cleared`, and the set's rights metadata names its source licence. | Inspection |
| FR-172-AC-3 | Against a synthetic licensed pack `example.invalid/std@1` installed in a temp `IX_HOME`, a skill run yields mappings only for clause ids present in the pack, and a source with no installed set yields no mapping and a not-mapped statement. | Demonstration |
| FR-172-AC-4 | No artifact or reply of the FR-172-AC-3 skill run contains the pack's clause-text sentinel. | Demonstration |
| FR-172-AC-5 | `SKILL.md` instructs the installed-set check, the not-mapped statement and the no-text rule. | Inspection |
| FR-172-AC-6 | The keys `nist/sp800-53@5.2.0` and `owasp/asvs@5.0.0` appear in this requirement, in spec-objects-security FR-014 and FR-017, and in `SKILL.md`. | Inspection |
| FR-172-AC-7 | No file shipped under `skills/spec-security-analysis`, and no fixture or expected output under `tests/fixtures/security-analysis`, contains a restricted-source control identifier (`CC<n>.<n>` or `A.<n>.<n>`). | Test |
| FR-172-AC-8 | The skill ships no file or directory whose name denotes a licence-restricted source reference. | Test |
| FR-172-AC-9 | No file of the skill links to, or names the path of, a licence-restricted source reference table, in any spelling of the link, and `SKILL.md` instructs ids of a restricted source only from an installed pack's clause set. | Test |
| FR-172-AC-10 | The synthetic pack fixture `example.invalid/std@1` under `tests/fixtures/security-analysis/packs`, installed in a temp `IX_HOME` with clause text added only inside that temp pack, is listed by `quoin module list` as a pack with no clause text in the output, and is not listed before it is installed. | Test |
| FR-172-AC-11 | `SKILL.md` instructs that pack clause text may be read locally but never written, and that a pack is the source's set only when its set key is the one the profile names. | Inspection |

## Dependencies

- **Upstream**: [FR-171](./FR-171-security-analysis-emits-typed-controls.md), [FR-137](./FR-137-detect-and-place-licensed-packs.md), [FR-138](./FR-138-packs-never-come-from-hosted-sources.md); quire-rs FR-083
- **Downstream**: [FR-173](./FR-173-facts-from-mappings.md); PLAT-1521 and PLAT-1522 author the sets; PLAT-1593, PLAT-1612, PLAT-1627 supply sets this one does not
