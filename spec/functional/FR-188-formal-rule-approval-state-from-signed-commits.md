---
id: FR-188
title: "Formal-rule approval state derived from signed commits"
type: FR
relationships:
  - target: "ix://agent-ix/quoin/StR-004"
    type: "traces_to"
  - target: "ix://agent-ix/engineering-assurance/FR-037"
    type: "depends_on"
---

# FR-188: Formal-rule approval state derived from signed commits

## Description

A formalized declaration is a QSL declaration that carries a `formalizes` marker
(the M1 link, quire-rs FR-093 and the VER-75 `qsl` adapter; this FR does not
restate it). Quoin SHALL report each formalized declaration as `approved` only
when a commit reachable from HEAD, verified by engineering-assurance as
`signed_valid` with a mapped principal and the meaning `formal-rule-approved`,
names the declaration in a `Formal-rule:` trailer and the declaration's text at
that commit equals its text at HEAD. Otherwise Quoin SHALL report `draft` with
exactly one reason from the closed set below.

This FR specifies the capability to verify signed approvals. It enables no
signing for this repository's commits, tags or releases and configures no
signer or release signing.

People accept risk. An agent can propose a formal rule and can never approve
one. The approval is the signed git commit; no state, digest or approval list
is stored. Origin: VER-98.

## Inputs

- `--repo <dir>` (default: the current directory).
- `--engineering-assurance <abs path>`: the engineering-assurance executable.
- `--allowed-signers <abs path>`: the allowed-signers file handed to
  `signature-verify`.
- `--json`: emit the JSON report instead of text.

## Outputs

`quoin formal-rules status` writes one report, sorted by `path`, then `label`.
With `--json` it is:

```json
{"protocol":"quoin.formal-rule-status/v1","rules":[{"symbol":"qsl:<path>::<label>","path":"…","label":"…","declaration_range":{"start":120,"end":340},"formalizes":["FR-012"],"state":"approved","reason":null,"approval":{"commit":"<sha>","principal":"…"}}]}
```

`declaration_range` is the half-open interval `[start, end)` of UTF-8 byte offsets,
into the file content at HEAD, of the declaration span the `qsl` adapter reports
for that declaration (the declaration span itself, without the marker comment
lines), with `start` below `end`. The offsets are computed from the content at HEAD at each run and are
persisted nowhere. `reason` is present only for `draft`; `approval` is present only for
`approved`; Exit status: `0` the report was produced; `2` usage error, including an
allowed-signers file engineering-assurance refuses
(`signature_verify_allowed_signers_invalid`); `3` engineering-assurance or git is
unavailable, or `signature-verify` refuses for any other reason. A refusal
produces no partial report; the engineering-assurance error code is written to
stderr.

## Behavior

- The symbol id of a declaration SHALL be `qsl:<repo-relative path>::<label>`,
  the VER-74 form.
- A declaration SHALL be `approved` when a commit C reachable from HEAD
  satisfies all of the following; the most recent such C is the approval of
  record.
  1. engineering-assurance `signature-verify` reports `status: signed_valid`,
     `signer.mapping: mapped` and a `principal`, for an SSH signature
     (engineering-assurance FR-036, FR-037) or a gitsign signature
     (engineering-assurance FR-040).
  2. `signed_off_meaning.presence` is `present` and its `values` are exactly
     `["formal-rule-approved"]`; a commit carrying a second, different meaning
     trailer does not satisfy this.
  3. C's message carries a `Formal-rule: qsl:<repo-relative path>::<label>`
     trailer naming this declaration. One commit MAY carry several such
     trailers.
  4. The declaration's text at C is byte-identical to its text at HEAD. Text
     is the marker comment lines plus the declaration span as the `qsl` adapter
     reports them when run on each revision's file content.
- Every other formalized declaration SHALL be `draft` with exactly one reason.
  The reason SHALL come from the most recent commit that names the declaration
  in a `Formal-rule:` trailer, taking the first condition of this closed,
  ordered set that holds; with no naming commit the reason is `no_approval`.
  1. `no_approval`: no reachable commit names the declaration.
  2. `approval_unsigned`: the naming commit carries no signature.
  3. `approval_bad_signature`: the signature does not verify against the
     commit.
  4. `approval_untrusted`: the signature verifies but the allowed-signers file
     does not accept the signer.
  5. `approval_unverifiable`: engineering-assurance reports `unverifiable`,
     including an accepted signature that yields no principal
     (engineering-assurance FR-037, reason `unrecognized_output`).
  6. `meaning_missing`: no `Signed-off-meaning:` trailer.
  7. `meaning_mismatch`: the meaning is present and its values are not exactly
     `["formal-rule-approved"]`.
  8. `changed_since_approval`: every other condition holds and the text at that
     commit differs from the text at HEAD.
- An edit to the marker lines or the declaration span after C SHALL yield
  `changed_since_approval`. A label rename or file move gives the declaration a
  new symbol id that no commit names, so it SHALL be `draft` with reason
  `no_approval` until a new approval names the new symbol id. Removing the
  `formalizes` marker SHALL remove the declaration from the report.
- An approval commit SHALL approve only the declarations its `Formal-rule:`
  trailers name.
- Quoin SHALL obtain trailers only from the `trailers` member of
  engineering-assurance `signature-verify` (engineering-assurance FR-037
  amendment), requested with `trailer_keys: ["Formal-rule"]`.
- An approval SHALL be a signature that engineering-assurance verifies against
  the allowed-signers file. The ix-flow gate acknowledgement is never an
  approval. Quoin does not inspect
  principals: that only people are listed in the allowed-signers file is the
  repository owner's control, not a check Quoin performs.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-188-CON-1 | Quoin SHALL contain no signature or cryptography code and add no cryptography dependency; verification is engineering-assurance `signature-verify` only | Security | Inspection |
| FR-188-CON-2 | Quoin SHALL write no state, digest or approval list to the repository | Integrity | Test |
| FR-188-CON-3 | Quoin SHALL list candidate commits with `git log --format=%H -E --grep='^Formal-rule: '` | Behavior | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-188-AC-1 | Given a fixture whose approval commit is SSH-signed by a mapped key with meaning `formal-rule-approved` and a `Formal-rule:` trailer naming the declaration, the report gives `approved` with that principal and commit | Test |
| FR-188-AC-2 | Given a fixture with no commit naming the declaration, the report gives `draft` with reason `no_approval` | Test |
| FR-188-AC-3 | Given an unsigned naming commit, the reason is `approval_unsigned` | Test |
| FR-188-AC-4 | Given a naming commit with a corrupted signature, the reason is `approval_bad_signature` | Test |
| FR-188-AC-5 | Given a valid signature by a key the allowed-signers file does not accept, the reason is `approval_untrusted` | Test |
| FR-188-AC-6 | Given a naming commit whose signature git accepts but yields no principal (engineering-assurance `unverifiable`, reason `unrecognized_output`), the reason is `approval_unverifiable` | Test |
| FR-188-AC-7 | Given a naming commit with two `Signed-off-meaning:` trailers, `formal-rule-approved` and another, the reason is `meaning_mismatch` | Test |
| FR-188-AC-8 | Given a valid signed commit with no `Signed-off-meaning:` trailer, the reason is `meaning_missing` | Test |
| FR-188-AC-9 | Given a valid signed commit with meaning `release-approved`, the reason is `meaning_mismatch` | Test |
| FR-188-AC-10 | Given a valid approval and one line of the declaration body edited afterwards, the reason is `changed_since_approval` | Test |
| FR-188-AC-11 | Given an approval commit whose `Formal-rule:` trailer names rule A, rule B in the same file is `draft` with reason `no_approval` | Test |
| FR-188-AC-12 | Given a label renamed after approval, the report gives the new symbol id `draft` with reason `no_approval` | Test |
| FR-188-AC-13 | Given a valid approval and an unrelated line of the same file edited afterwards, the declaration stays `approved` | Test |
| FR-188-AC-14 | Given two valid approval commits for one rule, the report names the more recent commit as the approval | Test |
| FR-188-AC-15 | Given a naming commit that fails several conditions, the single reason reported is the first in the closed order | Test |
| FR-188-AC-16 | The quoin workspace contains no signature or cryptography code, and `Cargo.toml` adds no cryptography dependency for this command | Inspection |
| FR-188-AC-17 | After a run over a fixture repository, the working tree and git history are unchanged | Test |
| FR-188-AC-18 | Given `--engineering-assurance` naming a missing executable, the command exits `3`; given a missing required flag, it exits `2` | Test |
| FR-188-AC-19 | Given a commit that only acknowledges a workflow gate and carries no signature, the declaration is not `approved` | Test |
| FR-188-AC-20 | Given an allowed-signers file engineering-assurance refuses with `signature_verify_allowed_signers_invalid`, the command exits `2` and writes no report; given any other `signature-verify` refusal, it exits `3` and writes no report | Test |
| FR-188-AC-21 | Given a declaration after a multi-byte character (for example `é`) in its file, `declaration_range` counts UTF-8 bytes, so slicing the file bytes at `[start, end)` yields exactly the declaration span, whether the declaration is `approved` or `draft` | Test |
| FR-188-AC-22 | Given any fixture, every `declaration_range` in the report has `start` below `end`, for `approved` and `draft` declarations alike | Test |

## Dependencies

- **Upstream**: engineering-assurance FR-037 (the `trailer_keys` amendment),
  FR-036 and FR-040; the M1 `formalizes` link (VER-74, VER-75).
- **Downstream**: [FR-189](./FR-189-formalize-requirement-workflow.md),
  [FR-190](./FR-190-formal-rule-analysis-claim-selection.md).
