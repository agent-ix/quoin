---
id: MP-220
title: CLI agent evaluation terminal-sentinel contract
type: MeasurementPlan
status: active
owner: engineering assurance
stage: branch-comparison
metric: cli_agent_eval_completion_rate
definition_version: cli-agent-evals.sentinel-contract-v1
relationships: []
---

# CLI agent evaluation terminal-sentinel contract

## Decision Use

Decide whether accepting only a standalone terminal marker prevents quoted
sentinel text from ending a live agent evaluation prematurely.

## Population

Two repeated Codex runs of the same Engineering Assurance existing-profile
scenario before the detector change and two repeated runs after it.

## Measure Definition

For each retained report, successful samples divided by total repeated samples.
The treatment effect is the treatment pass-rate fraction minus the baseline
pass-rate fraction.

## Collection Procedure

Run cli-agent-evals outside Quoin with the same host, fixture, suite, and repeat
count. Retain each emitted JSON report byte-for-byte. Quoin only adapts the
retained reports and does not invoke the runner or agent.

## Environment and Sampling

The host is Codex CLI 0.151.0 with its observed default model. The suite uses
fictional Engineering Assurance inputs. Live-agent nondeterminism, session
state, and omission of the required terminal marker remain uncontrolled.

## Interpretation

A changed pass rate is an observation, not a causal result. Equal pass rates,
limited repetition, uncontrolled live-agent behavior, and protocol failures
require a cause_not_established conclusion.

## Comparison and Enforcement

Compare only reports with the same scenario set, repeat count, suite digest,
fixture digest, and host version. Preserve failures and timeouts as observed;
do not replace them with constructed successes.
