---
id: FR-183
title: "quoin discharge --rmm reports per-clause compliance for facts recorded from mappings"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/FR-182"
    type: "depends_on"
  - target: "ix://agent-ix/quoin/FR-135"
    type: "depends_on"
---

# FR-183: quoin discharge --rmm reports per-clause compliance for facts recorded from mappings

## Description

The path from a typed control to a compliance row is `facts-from-mappings` (FR-182), then `record-discharge`
(FR-133), then `quoin discharge --repo --rmm` (FR-135). The report SHALL show each clause the mappings reach as
`Compliant`, `Tailored` or a `Not compliant` state without any change to the FR-135 view. This requirement fixes the
end-to-end result; FR-135's rules are unchanged.

## Behavior

- A clause answered by an `effective` control and attested SHALL render `Compliant`, with the control ids as its Evidence.
- A clause with a `scope_exclusion` and a recorded `delegated` disposition SHALL render `Tailored`, never `Compliant`.
- A clause whose controls are not effective SHALL render `Not compliant - undischarged`.
- Re-running the chain SHALL replace the stored fact of each clause it emits (FR-133).
- The chain SHALL carry no clause text and no score.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-183-AC-1 | Over a synthetic set with three binding clauses, one answered by an effective control, one excluded with a recorded approval, and one answered only by an `ineffective` control, `quoin discharge --repo --rmm` shows `Compliant` with the control id, `Tailored`, and `Not compliant - undischarged` respectively. | Test |
| FR-183-AC-2 | After the expiry instant given as `--as-of`, the first clause renders `Not compliant - attestation not current`. | Test |
| FR-183-AC-3 | The rendered matrix and the chain's intermediate files contain none of the set's clause-text sentinel and no percentage or count of compliant rows. | Test |
| FR-183-AC-4 | The same chain against a synthetic pack installed in a temp `IX_HOME` renders the same states by clause id. | Test |

## Dependencies

- **Upstream**: [FR-182](./FR-182-facts-from-mappings.md), [FR-135](./FR-135-requirements-mapping-matrix-view.md), [FR-133](./FR-133-discharge-facts-in-the-evidence-store.md)
- **Downstream**: none
