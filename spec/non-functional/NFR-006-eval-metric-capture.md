---
id: NFR-006
title: "The agent eval set captures efficiency metrics"
type: NFR
quality_attribute: functional_suitability
relationships:
  - target: "ix://agent-ix/quoin/TM-002"
    type: "references"
---

# NFR-006: The agent eval set captures efficiency metrics

## Statement

The agent-facing eval set SHALL record, per scenario, the latency, token usage,
tool-call count, validation-attempt count, and repeated-context-fetch count, so
that the efficiency with which an agent uses `quoin` is measurable and
comparable across runs.

## Scope

- Applies to: the agent-pty eval harness (`evals/`) and its matrix
  (`spec/evals.md`, TM-002).
- Operational context: evaluation runs against the real agent.

## Rationale

Unit tests prove command behaviour, but they cannot show whether an agent uses the
commands efficiently. Recording these metrics per scenario turns "did it work" into
"how well did it work", which is what guides interface and skill improvements.

## Measurement and Evaluation

| Metric                                                                                                     | Target | Threshold | Method   |
| ---------------------------------------------------------------------------------------------------------- | ------ | --------- | -------- |
| Required metrics recorded per scenario (latency, tokens, tool calls, validation attempts, context fetches) | 5 of 5 | 5 of 5    | Analysis |

## Verification

The eval harness is exercised and its result file inspected to confirm each
scenario records all five metrics from the real agent session transcript
(`evals/run.mjs`, reported in `evals/reports/latest.json`).

## Dependencies

- **Upstream**: the eval matrix [TM-002](../evals.md) and the harness in `evals/`.
- **Downstream**: none.
