---
name: spec-risk-complexity-analysis
description: Identify technical risks and volatility in requirements before tasking.
---

# Risk & Complexity Analysis

Surface the requirements that are most likely to fail, slip, or churn — so they can be sliced smaller, prototyped first, or hedged with extra verification.

## When to use

- After dependency, scope, and evidence analysis but before `spec-to-plan` decomposes tasks.
- After any major external change (new compliance regime, new SLA, new vendor).

## Risk dimensions

Each FR/StR/NFR is scored on two independent axes.

### Technical risk

Flag a requirement as **high risk** if it involves any of:

- **Novel technology** — first time this team or system uses it (new database engine, new protocol, new language runtime).
- **Concurrency / distributed coordination** — multi-writer state, consensus, leader election, exactly-once semantics.
- **Security or compliance constraint** — regulated data, cryptographic primitives, auth boundary changes.
- **Hard performance guarantee** — quantified latency/throughput/availability bounds (p99, RPO/RTO).

Otherwise medium (one of the above is tangentially involved) or low.

### Volatility

Flag a requirement as **high volatility** if it is likely to change due to:

- **Policy evolution** — regulatory or org policy not yet stable.
- **External contracts** — depends on a third-party API we don't control.
- **Product discovery** — UX or business rules expected to shift as users are observed.

Otherwise medium or low.

## Process

1. **Score** every FR/StR/NFR on both axes.
2. **Mitigate high-risk, low-volatility** items by spiking/prototyping early and adding contract/property tests.
3. **Mitigate high-volatility** items by slicing into smaller iterative tasks and avoiding deep dependencies on them.
4. **Cross-check failure domains** — see `spec-failure-domain-analysis` for extension, identity, purity, and topology gaps that *also* increase risk.
5. **Produce the risk register** below.

## Deliverable template

Record output in `spec/analysis/risk-register.md`.

```markdown
# Risk Register

| Req | Tech Risk | Volatility | Drivers | Mitigation |
|-----|-----------|------------|---------|------------|
| FR-018 | High | Low | Distributed lock, exactly-once payout | Spike + property tests + contract test against ledger |
| FR-022 | Medium | High | UX flow under product discovery | Slice into 3 iterative tasks; avoid downstream coupling |
| NFR-005 | High | Low | p99 < 50ms across 100rps | Load test as quality gate; budget allocated in FR-018 |
| FR-030 | Low | High | Tax rules change quarterly | Move rule table behind config; ship update path |

## Top hazards

1. FR-018 — distributed exactly-once payout (see mitigation above).
2. NFR-005 — perf budget owns FR-018's success; if perf fails, payout slips.

## Failure-domain gaps

See `spec-failure-domain-analysis` deliverable. Open gaps: none / list FR IDs.
```

## Acceptance Criteria

- Every FR/StR/NFR has both axes scored (no blanks).
- Every High on either axis has a named mitigation.
- The "Top hazards" section names the 3–5 items the team should review live before plan generation.
- Cross-reference to `spec-failure-domain-analysis` deliverable is present and current.

## Output contract

This skill emits one **`SpecReview`** under `reviews/YY-MM-DD-<slug>.md` with
`analysis: risk-complexity` in its frontmatter.

Stated here because nothing checked it. The module declares an `analysis`
vocabulary of eleven values; five of them named a skill that never said it
emitted under them, so a skill and its declared output were unlinked and drift
between the two was invisible. `tests/skill-contracts.test.ts` now asserts the
pairing in both directions.

The document must carry a `## Summary` and a `## Findings` table — the two
sections `SpecReview`'s `body_extraction` requires — and must pass
`quire validate`.
