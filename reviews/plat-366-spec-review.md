---
id: SR-20261111195249
title: "Spec review of quoin PR 812 (FR-036 AC-5/AC-7/AC-8 amended in place)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@3a2c4228; spec/functional/FR-036-architecture-conformance.md, spec/matrix.md"
review_set: subset
---

# Spec review of quoin PR 812

## Summary

Ticket: PLAT-366. Scoped to the FR-036 and matrix hunks of `git diff origin/main...HEAD` at
3a2c4228. `quire validate` on FR-036 passes. On spec/matrix.md, only the pre-existing TC-1943/TC-1944
tombstone rows fail, and this PR does not touch them.

Each amended AC was checked against behaviour. AC-7 is true: no workspace crate has a normal or build
edge onto `quoin-cli`. AC-8 is true today: `git` appears in measurement.rs, measurement/tamper.rs,
measurement/source_state.rs and campaign/source.rs, `ix-flow` in flow.rs, and `$EDITOR` in config.rs.
No production code spawns `quire`. NFR-007 still describes quire as emitted, not executed, so it is
consistent. AC-5's tool list matches the registry's `audit-script` claims exactly. Dropping the
per-language suite guidance is justified, because FR-036 has no such obligation and CON-2 only says
the method rides per-language tooling.

## Verdict

**PASS WITH FINDINGS (two low).** The amended ACs no longer describe the retired TypeScript tree, and
the history paragraph keeps the retracted claim visible. Two wording issues remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-036 prose says "A test over the workspace's resolved dependency graph and its Rust sources can tell them apart" and "a check that silently ignores what it cannot read is asserting over a set it cannot see". That presents TC-203 as the fix for regex misreads. TC-203 is a line-substring scan that silently skips code after a textual `#[cfg(test)]`, braced imports not led by `Command`, and unknown aliases (see SR-20261111195247 FND-001). The prose overclaims until the scan fails closed. Either fix the scan or state the limits | spec/functional/FR-036-architecture-conformance.md:81 |
| FND-002 | low | FR-036-AC-5 is compound. It states adapter selection, the list of claimed tools, the acceptance rule, and a rationale clause ("because the format names no producer…"). The rationale belongs in the description. Selection and acceptance could be two criteria so each binds its own test (TC-201 and TC-2900) | spec/functional/FR-036-architecture-conformance.md:112 |
