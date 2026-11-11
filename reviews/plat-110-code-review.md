---
id: SR-20261111185434
title: "Code review of quoin PR 807 (FR-131 plan-enumerated population)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@0daec931; rust/crates/quoin-measurement/src/{population_set.rs,plans/design.rs,types/plan.rs,validate/population.rs,verify/rows.rs,verify/reason.rs,verify/mod.rs,error.rs,intervention/intake.rs,lib.rs}, rust/crates/quoin-core/src/ops/measurement/taxonomy.rs, rust/crates/quoin-measurement/tests/tc_2400_population_set.rs"
review_set: subset
---

# Code review of quoin PR 807

## Summary

Ticket: PLAT-110. Scoped to `git diff origin/main...HEAD` at 0daec931. The Rust lane
(`rust-review`, with the repo's `.claude/skills/rust-style`) is folded in. Commands ran
through locked-build.sh with debug=0 and jobs=4: `cargo fmt --all --check` (clean),
`cargo clippy -p quoin-measurement --all-targets --all-features -D warnings` (clean), and
`cargo test -p quoin-measurement` (all passed, including the 8 tests in
`tc_2400_population_set`). A throwaway probe test measured the edge inputs cited below.
It was deleted and never pushed.

Clean: one shared check (`population_set::check`) serves both intake and verify, and each
side maps `SetFault` to its own code or reason. Ids are compared byte-exact, so a case or
whitespace variant (`C`, `c `) is refused as OUTSIDE-SET, and the real id is then missing.
Duplicate ids are MALFORMED. A convenient subset is PARTIAL and names the omitted ids. A
count that claims the whole set while the list is a subset is MALFORMED. Counts alone are
UNSTATED at intake and `population_unstated` (inconclusive) at verify. The `not_computed`
exemption cannot produce a pass, because verify returns `NoValue` for any non-measured
observation before it checks the set. The id lists are bounded by the existing byte ceilings
(4 MiB collection, 1 MiB plan), and `BTreeSet` keeps the check at O(n log n), so the lists
alone cause no CPU blow-up. The plan reader refuses unknown members, so the object is
closed. The new codes map to BadRequest, and both count asserts (33 -> 35) were updated
deliberately. Opt-in (the check runs only under a declared `population_set`) is the lead's
ruling for this slice and is not a finding.

## Verdict

**FAIL (one high, two medium, one low).** The intake does not apply the check when
`population` is present but is not an object, so FR-131-AC-3 is violated on intake, although
verify still never accepts the observation. A non-numeric stated count escapes the
count-equality rule. The PARTIAL/OUTSIDE findings name every id, so the error output grows
as observations x set size.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Under a plan declaring `population_set` but no `minimum_population` and no `repetitions` > 1, a measured observation whose `population` is a string, number, array, bool or null is ADMITTED at intake (probed: `"a,b,c"`, `3`, `["a","b","c"]`, `true`, `null` all returned Ok). The non-object branch returns before `set_finding` runs, and it reports malformed only when a minimum or repetitions is declared. This violates FR-131-AC-3 and makes intake disagree with verify, which reads the observation as `population_unstated`. Under a declared set, a non-object `population` must be refused (MALFORMED, or UNSTATED for null) | rust/crates/quoin-measurement/src/validate/population.rs:70-79 |
| FND-002 | medium | The count-equality rule reads the lenient typed parse, so a stated count that is not a number is never compared with its list. `{"examined":"1","matched":"1","examined_ids":["a","b","c"],"matched_ids":["a"]}` is admitted, which contradicts FR-131-AC-3 ("disagrees with a stated count is refused as QM-POPULATION-MALFORMED"). Verify then reads the observation as unstated. Read the raw `examined`/`matched` members under a set, and refuse a stated count that is not a whole number | rust/crates/quoin-measurement/src/validate/population.rs:179-180 |
| FND-003 | medium | `SetFault::describe` joins every outside or missing id into the finding, and intake accumulates one finding per observation with no ceiling. Measured: a 2000-id plan and 2000 observations with empty lists, which is 505 KB of observations, produce 40.3 MB of findings (80x). Within the 4 MiB collection and 1 MiB plan ceilings this grows to multiple GB. This breaks rust-style's "every accumulator from the stream has a ceiling". Name the first N ids and add "and M more", or bound the number of findings | rust/crates/quoin-measurement/src/population_set.rs:129-138 |
| FND-004 | low | An observation that states the lists but no `examined`/`matched` counts is admitted at intake (tc_2404 relies on this), but verify rejects it as `population_unstated` (inconclusive), because `examined` is required before the set check. Under a declared set the counts can be derived from the lists. Either require the counts at intake under a set, or derive them in verify, so that intake and verify agree | rust/crates/quoin-measurement/src/verify/rows.rs:179-196 |

## Examined scope

```yaml
clean: false
scope:
- {id: population_set::check, path: rust/crates/quoin-measurement/src/population_set.rs, role: examined, excerpt: "examined_ids exactly the set, matched_ids subset, counts equal list length; order: unstated, malformed, outside, partial"}
- {id: population_set_from, path: rust/crates/quoin-measurement/src/plans/design.rs, role: examined, excerpt: "object with only `ids`, non-empty list of distinct non-empty strings, else QM-PLAN-INVALID"}
- {id: PopulationSet, path: rust/crates/quoin-measurement/src/types/plan.rs, role: examined, excerpt: "pub struct PopulationSet(BTreeSet<String>)"}
- {id: set_finding, path: rust/crates/quoin-measurement/src/validate/population.rs, role: examined, excerpt: "let set = design?.population_set.as_ref()?; ... population_set::check(set, member(EXAMINED_IDS), member(MATCHED_IDS), stated.examined, stated.matched)"}
- {id: assess, path: rust/crates/quoin-measurement/src/verify/rows.rs, role: examined, excerpt: "if let Some(set) = &design.population_set { population_set::check(...).map_err(...)?; }"}
- {id: Reason::verdict, path: rust/crates/quoin-measurement/src/verify/reason.rs, role: examined, excerpt: "PopulationOutsideSet | PopulationPartial => Verdict::Reject"}
- {id: measurement_code, path: rust/crates/quoin-core/src/ops/measurement/taxonomy.rs, role: examined, excerpt: "Code::PopulationOutsideSet | Code::PopulationPartial => BadRequest; ALL.len() 35"}
- {id: MeasurementErrorCode, path: rust/crates/quoin-measurement/src/error.rs, role: examined, excerpt: "QM-POPULATION-OUTSIDE-SET, QM-POPULATION-PARTIAL"}
- {id: tc_2400_population_set, path: rust/crates/quoin-measurement/tests/tc_2400_population_set.rs, role: examined, excerpt: "TC-2400..TC-2407"}
- {id: MAX_COLLECTION_BYTES, path: rust/crates/quoin-core/src/ops/measurement/wire.rs, role: context_only, excerpt: "pub const MAX_COLLECTION_BYTES: usize = 4 * 1024 * 1024;"}
```
