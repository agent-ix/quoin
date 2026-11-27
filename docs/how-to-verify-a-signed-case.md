---
title: Verify a signed assurance case
type: how-to
---

To export a signed GSN case, run these commands from its project repository root.

## Before you start

Run these commands from the project repository root. Use a Quoin build whose
`quoin assurance --help` lists `--case-approval-tag`,
`--case-approval-verifier`, and `--case-approval-allowed-signers`. Install
Engineering Assurance 0.11.0, Git with SSH signing support, Python 3, and
`rfc8785` 0.1.4. The project must already produce a valid GSN case with
`quoin assurance --gsn`. Use one terminal so the digest variable remains set.

## Procedure

1. Move to the project root and set the signing and trust-file paths. The
   allowed-signers file must contain the principal and public key for the
   signing key.

```sh
cd /path/to/project
KEY="$HOME/.ssh/case-review"
ALLOWED_SIGNERS="$PWD/allowed_signers"
EA="/absolute/path/to/engineering-assurance"
printf '%s %s\n' 'reviewer@example.test' "$(cat "$KEY.pub")" > "$ALLOWED_SIGNERS"
```

2. Create a temporary Python environment for canonicalizing the case digest.

```sh
python3 -m venv .case-approval-venv
. .case-approval-venv/bin/activate
python -m pip install rfc8785==0.1.4
```

3. Export the unsigned case and calculate its RFC 8785 digest.

```sh
quoin assurance --repo . --gsn > case.json
digest=$(python - case.json <<'PY'
import hashlib
import json
import pathlib
import sys
import rfc8785

case = json.loads(pathlib.Path(sys.argv[1]).read_text())
print('sha256:' + hashlib.sha256(rfc8785.dumps(case)).hexdigest())
PY
)
```

4. Create an annotated SSH-signed tag with the case identity and acceptance
   meaning.

```sh
git -c gpg.format=ssh -c user.signingkey="$KEY" tag -s case-approved \
  -m 'Review assurance case' \
  -m 'Approved-Record-Id: quoin.gsn-case/v1' \
  -m "Approved-Record-Digest: $digest" \
  -m 'Signed-off-meaning: assurance-case-accepted'
```

5. Ask Quoin to verify the selected tag with Engineering Assurance.

```sh
quoin assurance --repo . --gsn \
  --case-approval-tag case-approved \
  --case-approval-verifier "$EA" \
  --case-approval-allowed-signers "$ALLOWED_SIGNERS" \
  > approved-case.json
```

## Expected output

The exported GSN contains a `signed_acceptance` object with fields like these:

```json
{
  "signed_acceptance": {
    "record_id": "quoin.gsn-case/v1",
    "record_digest": "sha256:<case-digest>",
    "tag": "case-approved",
    "approver": "reviewer@example.test",
    "meaning": ["assurance-case-accepted"],
    "signer_source": "allowed_signers",
    "trusted_at_missing": "ssh_signature"
  }
}
```

The record identifies the case digest, tag, mapped approver, meaning, and
signing time. For offline SSH verification, `trusted_at_missing` is
`ssh_signature` because the signature does not attest to a trusted signing
time.

## Verify the output

Check the identity and approval fields before sharing the export:

```sh
jq -e '.signed_acceptance.record_id == "quoin.gsn-case/v1" and .signed_acceptance.record_digest == $digest and .signed_acceptance.tag == "case-approved" and .signed_acceptance.approver == "reviewer@example.test" and .signed_acceptance.meaning == ["assurance-case-accepted"] and .signed_acceptance.trusted_at_missing == "ssh_signature"' --arg digest "$digest" approved-case.json
```

The command prints `true` when every check passes. A missing or rejected
approval leaves the case export available and prints a reason to stderr.
