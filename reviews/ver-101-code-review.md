---
id: SR-20261116075205
title: "Code review — VER-101 formal rule approval state and workflow (quoin PR 950)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin; rust/crates/quoin-cli/src/formal_rules.rs, rust/crates/quoin-cli/src/flow/provider.rs, rust/crates/quoin-quire/src/formal_rules.rs, rust/crates/quoin-cli/src/main.rs, skills/formalize-requirement/"
review_set: subset
relationships:
  - target: "ix://agent-ix/quoin/FR-188"
    type: reviews
  - target: "ix://agent-ix/quoin/FR-189"
    type: reviews
---

## Summary

Reviewed frozen PR `agent-ix/quoin#950` at `15244a674e8a29f226c3f04549b51f181ef000b3` against freshly fetched `origin/main`. Rust review was folded into this code-review artifact. The PR is GitHub-mergeable, but has three contract defects: it reads current rule text from the worktree instead of HEAD, fails to preserve the specified allowed-signers refusal exit code, and can emit a reason outside FR-188's closed set. The released quire-rs `v0.61.0` tag is used; no signature implementation or cryptographic dependency was introduced, and the reviewed paths do not write repository state or configure signing.

## Verdict

**FAIL** — FND-001 and FND-002 violate FR-188 observable behavior; FND-003 emits a reason outside the closed contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Current declaration bytes and ranges come from the working tree, although FR-188 defines both at HEAD; an uncommitted marker/declaration edit changes the reported state without changing HEAD. | rust/crates/quoin-cli/src/formal_rules.rs:160-161 |
| FND-002 | medium | `signature-verify` nonzero exits are collapsed into a generic unavailable error, so `signature_verify_allowed_signers_invalid` exits 3 instead of the required usage code 2. | rust/crates/quoin-cli/src/formal_rules.rs:355-359; rust/crates/quoin-cli/src/main.rs:93-98 |
| FND-003 | medium | A `signed_valid` result without mapped principal is reported as `approval_unmapped`, which is outside FR-188's closed reason set; this case must be `approval_unverifiable`. | rust/crates/quoin-cli/src/formal_rules.rs:278-284 |

## Detail

### FND-001 — status is based on uncommitted worktree contents

`status()` passes the repository root directly to `quoin_quire::formal_rules::extract` for the current rules. That adapter reads files from disk. The candidate approvals are reachable from `HEAD`, and historical versions are extracted from git trees, but the comparison target is therefore the worktree, not the file content at `HEAD`. If a user edits a formalized declaration or its marker without committing, the report changes to `changed_since_approval` even though FR-188 says the source text and `declaration_range` are computed from content at `HEAD`. This also makes status non-repeatable for a fixed HEAD. Extract the current rules from HEAD's tree (or otherwise read exact HEAD bytes) and add a regression test with a dirty worktree.

### FND-002 — allowed-signers refusal gets the wrong exit status

`verify()` returns the same `Err(String)` for every nonzero engineering-assurance process exit. `run()` maps status errors through `ShellError::unavailable`, whose outcome is `Refused` (exit 3). FR-188 AC-20 explicitly distinguishes `signature_verify_allowed_signers_invalid` (exit 2) from any other refusal (exit 3), and requires no report for either. Preserve the EA error code and map that refusal to the invalid/usage outcome; test both refusal classes and absence of stdout report.

### FND-003 — draft reason outside closed set

For `status: signed_valid`, a signer whose `mapping` is not `mapped` or whose principal is absent enters the `approval_unmapped` branch. FR-188 enumerates the complete reason set and explicitly classifies an accepted signature with no principal as `approval_unverifiable`. `approval_unmapped` is not a valid output and makes clients unable to rely on the closed reason vocabulary. Map this branch to `approval_unverifiable` and update the unit test that currently expects `approval_unmapped`.

## Rust review

Reviewed the Rust additions and their boundaries, process handling, error propagation, parsing, byte ranges, and tests. No cryptographic implementation, signature verification, or signature dependency was added; verification delegates to the explicit engineering-assurance executable and explicit allowed-signers path. Request trust fields and the optional closed gitsign tuple are passed explicitly. No review finding requires state writes, local signing, or repo/tag/release signing. The three findings above are the Rust review findings. Coder-reported focused tests, formatting, and clippy passed; aggregate gate remains lead-owned and was not run in this review.

## Review Scope

| Unit | Role | Result |
| --- | --- | --- |
| FR-188 status inputs, output, approval matching and reason precedence | examined | FND-001, FND-002, FND-003 |
| FR-189 trust request validation and explicit pass-through | examined | No additional code-review finding |
| signature, state, cryptographic dependency, release-signing boundaries | examined | No finding |
| Rust process/error/test behavior | examined | Findings recorded above |
| released quire-rs dependency | examined | `v0.61.0` tag is used |
