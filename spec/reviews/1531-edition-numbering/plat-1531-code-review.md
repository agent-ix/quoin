---
id: SR-20261113143521
title: "Code review of the PLAT-1531 security-analysis control renumbering"
type: SpecReview
analysis: code-review
review_set: subset
scope: "agent-ix/quoin; PR #873 (git diff origin/main...plat-1531-edition-numbering at 1c8cb730): skills/spec-security-analysis/SKILL.md, assets/security-analysis-example.md, references/component-type-mapping.md, references/standards-index.md, references/nist-800-53/SA-SystemServicesAcquisition.md, references/nist-800-53/SI-SystemInformationIntegrity.md, references/owasp-asvs/*"
---

# Code review of the PLAT-1531 security-analysis control renumbering

## Summary

Ticket: PLAT-1531. The change is skill reference content only; no code or tests changed.

Checked independently against the official catalogues, fetched with `gh api`: NIST OSCAL
`usnistgov/oscal-content` `nist.gov/SP800-53/rev5/json/NIST_SP-800-53_rev5_catalog-min.json` (metadata
version 5.2.0, 1196 controls and enhancements), and `OWASP/ASVS` `5.0/docs_en/OWASP_Application_Security_Verification_Standard_5.0.0_en.csv`.

- Every NIST id cited anywhere under `skills/spec-security-analysis` exists in Rev 5.2.0 and none is withdrawn.
- RA-5(11) is "Public Disclosure Program", SR-3 is "Supply Chain Controls and Processes", SI-2(2) is
  "Automated Flaw Remediation Status". Each matches its row (vulnerability disclosure, supply chain,
  automated patch status). The replaced SA-15(13) "Logging Syntax", SA-24 "Design For Cyber Resiliency"
  and SI-2(7) "Root Cause Analysis" did not.
- Every ASVS id cited exists in 5.0.0, including all ids in the example, `component-type-mapping.md`
  and every Mapping Guidance line. Every `x.y.x` range names a real 5.0.0 section.
- Each of the 17 chapter packs lists exactly the 5.0.0 requirement set of its chapter. Section title and
  L1/L2/L3 marks agree with the CSV for every row. Each pack heading is the 5.0.0 chapter name.
- Each Mapping Guidance line was read against the CSV requirement text. Each cited requirement covers the
  line's meaning (for example 8.2.2 data-specific access, 16.2.5 sensitive data in logs, 6.2.12 breached
  passwords, 15.1.2 SBOM).
- No 6-word run of any ASVS requirement description appears in any file under the skill. Packs carry ids,
  section titles and levels only. The CC BY-SA 4.0 attribution is in `owasp-asvs/index.md`.
- The diff does not touch `soc2/`, `iso27001/` or `gdpr/`. No SOC 2, ISO or GDPR number changed.
- A repo-wide grep finds no reference to the 14 deleted pack file names, "V1-V14" or "14 verification areas".
  The dropped rows (threat-model/SDLC, malicious code) have no 5.0.0 chapter, so dropping them is right.
- The diff adds no roadmap or research content.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `references/nist-800-53/index.md` still has the heading "NIST SP 800-53 Rev 5". The PR changes the example's "Rev 5" to "Rev 5.2.0" so the skill cites one revision string. FR-171 says the skill SHALL cite NIST SP 800-53 as Rev 5.2.0. This heading is the one place left that does not. | skills/spec-security-analysis/references/nist-800-53/index.md:1 |

## Verdict

Mergeable once FND-001 is fixed or dispositioned. Every cited id resolves in the official catalogue and the
re-pointed controls match their rows.
