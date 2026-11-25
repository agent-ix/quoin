# PLAT-1644 / PLAT-1447 / PLAT-1645 / PLAT-1850 focused run

- Revision: `5c664afe5f2125f3e42ae26eebebdc7b4ca00d7b`
- Ran: 2026-11-25 01:09:18 UTC
- Target: `/home/peter/dev/quoin/.worktrees/gsn-implementation-batch/target`
- Wrapper: `/home/peter/dev/worktrees/locked-build.sh`
- Exit status: 0
- Result: 16 passed, 0 failed
- Raw output: `/tmp/quoin-gsn-focused-round1.log`

Exact command:

```sh
/home/peter/dev/worktrees/locked-build.sh /home/peter/dev/quoin/.worktrees/gsn-implementation-batch/target bash -lc 'cd /home/peter/dev/quoin/.worktrees/gsn-implementation-batch/rust && cargo test --locked -p quoin-assurance --lib gsn::tests && cargo test --locked -p quoin-assurance --test case_graph mitigated_risks_are_sorted_context_and_missing_targets_stay_open && cargo test --locked -p quoin-assurance --test case_rendering assurance_context_renders_under_its_owning_claim && cargo test --locked -p quoin-cli --bin quoin gsn_refusal_tests && cargo test --locked -p quoin-cli --test tc_1018_assurance_unbacked_claim explicit_gsn_export_is_supported_and_conflicts_with_json'
```

| Focused target | Passed | Failed |
| --- | ---: | ---: |
| `quoin-assurance --lib gsn::tests` | 11 | 0 |
| `case_graph mitigated_risks_are_sorted_context_and_missing_targets_stay_open` | 1 | 0 |
| `case_rendering assurance_context_renders_under_its_owning_claim` | 1 | 0 |
| `quoin-cli --bin quoin gsn_refusal_tests` | 2 | 0 |
| `tc_1018_assurance_unbacked_claim explicit_gsn_export_is_supported_and_conflicts_with_json` | 1 | 0 |
