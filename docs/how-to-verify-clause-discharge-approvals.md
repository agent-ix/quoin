---
title: Verify approvals during clause discharge
type: how-to
---

Use this procedure when a clause set requires an approval before a direct fact can discharge a clause.

## Before you start

Install Quoin 0.37.0 and the Engineering Assurance verifier. Use a Quoin module that declares the clause set and a binding report for its exact authority, set, and version. The approval-required clauses must declare `approvalRoles`.

Prepare `discharge-facts.json` with one fact for each clause. A direct fact includes evidence references, an attestation, and an `approvalRef` that identifies the signed approval. Configure the verifier and its allowed-signers file to trust the approval signer.

## Record the facts

1. From the project root, store the facts with the binding report and approval verifier:

```sh
quoin evidence record-discharge \
  --repo . \
  --binding binding.json \
  --facts discharge-facts.json \
  --engineering-assurance /absolute/path/to/engineering-assurance \
  --allowed-signers /absolute/path/to/allowed_signers \
  --json
```

The command stores the attested facts under `spec/evidence/discharge/`. It verifies approval references for clauses whose set declares approval roles.

## Build the discharge report

2. Evaluate the binding report at an explicit instant with the same verifier and trust file:

```sh
quoin discharge \
  --repo . \
  --binding binding.json \
  --as-of "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --engineering-assurance /absolute/path/to/engineering-assurance \
  --allowed-signers /absolute/path/to/allowed_signers \
  --json
```

## Expected output

For a verified direct fact, the JSON report includes the verifier's approver and preserves the approval reference:

```json
{
  "clauseId": "CTRL-1",
  "verifiedApprover": "reviewer@example.com",
  "fact": {
    "approvalRef": "approval-tag"
  }
}
```

For a clause with no approval roles, Quoin preserves a supplied reference without adding `verifiedApprover`. A missing or invalid approval leaves the clause open with a reason.

## Verify the result

Read `binding.direct` and `binding.open` in the report. Check that each approval-required direct fact names the approver returned by Engineering Assurance, and that open clauses state why the fact did not discharge them.
