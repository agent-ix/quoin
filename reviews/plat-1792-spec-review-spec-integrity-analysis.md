---
id: SR-20261117175273
title: "Integrity review of the PLAT-1792 FR-185 amendment"
type: SpecReview
analysis: integrity
scope: "agent-ix/quoin; spec/functional/FR-185-in-toto-test-result-export.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quoin/FR-185", type: references }
---

## Summary

Reviewed the FR-185 amendment in PR #1014 for internal consistency and for agreement with the code. Ticket: PLAT-1792. The amendment adds three bullets: Quoin does not verify the DSSE signature, the population is the set of `criterion` members, and runs are taken by `recordPath`. Several unamended SHALLs now contradict those bullets.

## Verdict

**CHANGES REQUESTED.** The new bullets match the code. Older SHALLs and FR-185-AC-12 still require Quoin itself to refuse an invalid-signature or untrusted envelope, and the code accepts any non-empty `signatures`. The latest-run rule contradicts the new `recordPath` rule. The `criterion`-row shape does not match the canonical Quire matrix that FR-186 says the payload carries.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The amendment says Quoin does not verify the DSSE signature. Unamended SHALLs and AC-12 still require Quoin to refuse provenance that "has an invalid signature, has an untrusted source identity" (l.121), to refuse "when provenance authenticity ... cannot be established" (l.134), and to emit PASSED only from "authenticated" provenance (l.177). The code accepts `signatures: [{"sig":"AAAA"}]`, and tests tagged AC-12 pass, so the spec claims a refusal that the code does not perform. | spec/functional/FR-185-in-toto-test-result-export.md:119-135,177-179,210 |
| FND-002 | medium | l.64 still requires selecting "the latest run for each suite by greatest lexical timestamp", while the new l.97 bullet takes a suite's run only from the provenance `recordPath`. The code follows l.97 and ignores a newer run in the store. The two SHALLs contradict each other. | spec/functional/FR-185-in-toto-test-result-export.md:64-66,97-100 |
| FND-003 | medium | The population is "the set of non-empty string `criterion` members of the `testMatrix` rows" (l.94). FR-186 sets `testMatrix` to "the complete canonical Quire Test Matrix", but `quire matrix --format json` rows are `{document, id, criteria:[{id, status, ...}]}` and carry no `criterion` member, so a verbatim canonical matrix is refused. No "active" filter exists, although l.70 says "every active obligation" and this repository's matrix has 36 `retired` criteria. | spec/functional/FR-185-in-toto-test-result-export.md:70-72,94-96; rust/crates/quoin-evidence/src/export/provenance.rs:131-142 |
| FND-004 | low | The exit-code bullet lists exit 2 for usage or an invalid subject and exit 3 for the store, but does not state the exit code of the new `evidence_provenance_invalid` refusal. The code exits 2, and the revision-mismatch case also uses exit 2. | spec/functional/FR-185-in-toto-test-result-export.md:183-186 |

## Recommended fixes

- FND-001: rewrite l.119-135, l.177 and AC-12 so that signature and trust refusal are assigned to the Engineering Assurance verifier (FR-069). Quoin's refusals should be limited to shape, digest and revision. Retag the tests.
- FND-002: replace the latest-run bullet with the `recordPath` rule, or scope the latest-run rule to non-provenance readers.
- FND-003: specify the exact `testMatrix` row shape in FR-186 and FR-185, either the canonical nested form (parse `criteria[].id`) or a flattened `criterion` row. Define "active", for example as status not `retired`, and filter on it.
- FND-004: state the exit code for `evidence_provenance_invalid`.

## Scope

- FR-185 Behavior bullets l.56-187 (examined)
- FR-185-AC-10, FR-185-AC-11, FR-185-AC-12 (examined)
- FR-186 payload bullets l.71-120 (context_only)
