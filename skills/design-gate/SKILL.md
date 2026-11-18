---
name: design-gate
description: Propose and check a data-flow design, revise bounded denials, and leave acceptance to a person.
metadata:
  ix-flow-workflows: ./workflows
  ix-flow-invariant-provider:
    command: quoin
    args: [workflow-invariants]
    request-protocol: quoin.workflow-invariants/v1
    result-protocol: quoin.workflow-invariants-result/v1
    invariants:
      - design_edit_targets_safe
      - dataflow_rules_pass
      - design_gate_retry_allowed
      - design_gate_retries_exhausted
      - escalated_acceptance_preserves_denial
      - human_acceptance_required
---

# Design Gate

Use this skill for one architecture proposal that must satisfy the selected
modules' flow rules. The proposer is untrusted. Only the `quire` report decides
whether the design passes, and only a person's HITL decision accepts or rejects
it.

## Start the run

From the repository root, ask Quoin to start the workflow and seed its
protected context through the ix-flow host API:

```sh
quoin design-gate --path <absolute-path-to-this-skill-directory> --id <run-id> --target . --json
```

Record the design root relative to the repository root, every rule-module root
as an explicit absolute path, and the absolute `quire` executable path in the
`request` interview. The module list replaces ambient module discovery. Do not
start with a module directory that is writable by the proposer.

Quoin owns run creation and supplies `quoin_context` as a system-owned item
through a service runner. It sets the context `id` to the engine run id and
derives `repo_root`, `config_root`, and `targets` from the launch environment.
The agent cannot add or update this item. The invariant provider keys its
private evaluator-result state with the immutable engine run identity and uses
the context's configuration root only as the storage location.

The request item is:

```json
{
  "design_root": "<repository-relative-design-root>",
  "module_roots": ["<absolute-rule-module-root>"],
  "quire_executable": "<absolute-quire-executable>"
}
```

## Propose and revise

The design agent may read the design and rule inputs. It submits a `design_edit`
item containing relative paths and replacement content; the workflow provider
applies the batch using the trusted design root and module roots in the request.
It checks every target before writing any of them. A target outside the design
root or inside a rule module returns `rules_modified` before any write. Do not
give the proposer a general file-write tool for the design repository or any
rule module.

The proposer may return only design edits. Do not let it change the module
manifest, `required_relations`, `traceability.flow_rules`, or declared rule
data. The edit tool does not record file lists, snapshots, digests, or version
information.

After the initial proposal, advance to `checking` and follow the transition
allowed by the evaluator result. `dataflow_rules_pass` invokes `quire` with the
configured scope and every explicit module root. On a denied result, its failed
outcome includes the full report and current attempt. The invariant provider
records valid evaluator results in workflow state outside the design
repository; it does not trust proposer supplied `design_gate_check` items. A
missing, malformed, or unusable report surfaces `dataflow_check_unavailable`
and leaves the run in `checking`; it does not consume an attempt or retry.

Run the checker with the workflow's exact configured paths:

```sh
quire relations --scope <absolute-design-root> --flow --json \
  --module <absolute-module-root> [--module <next-absolute-module-root> ...]
```

Use the returned JSON only. The `dataflow_rules_pass` invariant passes only when
the report schema is `quire.flow-report/v1`, all required report members are
valid, and `denials` is empty. Each denial is sent back with its rule, check,
subject, matched facts, resolved data categories, and ordered remediation
details. Warning severity still denies this proposal.

For a valid denial before attempt 4, advance `checking` to `revising`, submit a
revised design, and then advance `revising` to `checking`. N = 3 means the
initial attempt plus three retries. After the fourth valid report with
denials, advance directly from `checking` to `escalated` without another
proposer retry. A valid report with no denials reaches `passed`, even on
attempt 4.

If the design agent cannot start or returns no usable edit, the edit invariant
returns `design_agent_unavailable`. Keep the run in its current `proposing` or
`revising` phase; an operator may resume or end it. Do not count this as a
denial or evaluator failure.

## Human decision

The `passed` and `escalated` phases both wait for a person at the HITL gate.
Show the person the last valid flow report and any unresolved denials. An
acceptance from `escalated` records a human decision only: keep the evaluator
result as `dataflow_denied`. The design agent cannot acknowledge or bypass the
gate. `accepted` and `rejected` are terminal.

This workflow does not create analysis evidence or signed records and does not
invoke signed-record verification. It does not enable signing on commits,
tags, or releases, configure a signer, or request signing keys.
