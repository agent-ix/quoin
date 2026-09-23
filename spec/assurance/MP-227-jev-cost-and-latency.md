---
id: MP-227
title: Jev lens cost, latency and throughput per pass
type: MeasurementPlan
status: active
owner: quoin-maintainers
stage: branch-comparison
metric: jev.pass_cost
definition_version: jev.cost-latency-v1
relationships: []
---

# Jev lens cost, latency and throughput per pass

## Decision Use

Decide whether a lens runs on every commit, on every PR, or on demand.

## Population

One sequential pass over the lens's corpus, plus one burst of concurrent requests.

## Measure Definition

Input and output tokens, wall-clock, per-request latency mean, p50, p90 and max, sequential and concurrent requests per second, definition `jev.cost-latency-v1`.

## Collection Procedure

Run the lens's `live-api` test through `quoin_jev::client::production`.
Retain the raw responses, question set, corpus and context revision, reported
classifier, token counts and timestamp. Dimension every observation by `lens`
and `variant`.

## Environment and Sampling

One pass is every item in the lens's labelled corpus, sent once. A transport
failure aborts the pass; it is never scored as a wrong answer.

## Interpretation

A measure only. A slow run must never fail a gate.

## Comparison and Enforcement

This plan assigns no verdict and no bar.
