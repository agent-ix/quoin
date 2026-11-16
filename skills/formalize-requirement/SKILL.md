---
name: formalize-requirement
description: Draft and type-check one QSL rule for a requirement.
metadata:
  ix-flow-workflows: ./workflows
  ix-flow-invariant-provider:
    command: quoin
    args: [workflow-invariants]
    request-protocol: quoin.workflow-invariants/v1
    result-protocol: quoin.workflow-invariants-result/v1
    invariants:
      - qsl_check_passes
      - formalization_retries_exhausted
      - analysis_consistent_with_approved
      - formal_rule_approved
---

Use one requirement or acceptance-criterion id as the input. Draft exactly one
`.native` file at the requested repository-relative path. Put
`// formalizes: <id>` immediately above the declaration that states the rule.
Do not edit the requirement Markdown.

Run the `formalize` workflow with the requirement id, draft path, the absolute
engineering-assurance executable path, and the absolute allowed-signers path.
These trust inputs are part of the recorded `formalization_request`; the
approval invariant does not read them from environment or repository config.
If using gitsign, patch the request item with one `gitsign` object holding
exactly `certificate_identity`, `certificate_oidc_issuer`, and `principal`.
The workflow invokes the configured `FSL_DRIVER_BIN` for `fsl check` and
`QUIRE_DRIVER_BIN` for Quire's `trace` command. A draft passes only when both
tools confirm a clean type check and a bound marker for the exact input id.

When the check fails, keep the failure in the run as a `formalization_failure`
item with its diagnostic, then revise the draft. Count consecutive failures
that have not been cleared. After any clean check, clear each prior failure
item with this command for that failure item's id (find ids with `ix-flow
status <run-id>`):

```sh
ix-flow update-item <run-id> formalization_failure <item-id> --patch '{"cleared":true}'
```

This preserves its diagnostic while resetting the retry streak if analysis
later asks for another revision. After the fourth
consecutive failure, transition to `stopped` and retain the uncleared
diagnostics in the final report. Do not retry after that transition.

After a clean check, record the exact `qsl:<repo-relative path>::<label>`
symbols declared in the checked draft in the request item's `drafted_symbols`
field using `ix-flow update-item`. Keep that set unchanged through analysis
and the approval gate; the approval invariant requires it to be non-empty and
checks every listed symbol.

This workflow drafts proposals only. It does not approve a rule or create a
signed record. The approval transition remains a person-controlled `hitl`
gate; never enable signing on a repository or request signing keys.
