---
id: SR-20261113000906
title: "Code review of quoin PR 838 (quoin soup promote, FR-141)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@e4fa4c89; rust/crates/quoin-cli/src/soup.rs, rust/crates/quoin-cli/src/soup/plan.rs, rust/crates/quoin-cli/src/help.rs, rust/crates/quoin-cli/src/main.rs, rust/crates/quoin-cli/src/tests.rs, rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs"
review_set: subset
---

# Code review of quoin PR 838

## Summary

Ticket: PLAT-1515. Scoped to `git diff origin/main...HEAD` at e4fa4c89 (one commit, 7 files).
The Rust lane (`rust-review`, with quoin's repo `rust-style` as the higher authority and the
ecosystem `rust-style` as baseline) is folded in. All cargo commands ran through locked-build.sh
with debug=0 and one scratch target dir. `cargo test -p quoin-cli --test tc_1515_soup_promote`
passed 11 of 11, using `SPEC_OBJECTS_SECURITY_DIR` set to a clone of
agent-ix/spec-objects-security main at #45. The `--bin quoin` unit tests passed 64 of 64,
including tc_1515_001..004, tc_373 and tc_1650. `tc_retained_help_capture` passed 6 of 6.
`cargo clippy -p quoin-cli --all-targets -D warnings` and `cargo fmt --check` are clean. Four
defects were reproduced with the built binary (see Findings).

The command does what the brief asks for. Components come only from
`quoin_evidence::adapters::parse_sbom`, and the purl parse reads the adapter's symbol, not the
SBOM. The soup_record type is found through `catalog.load` and the body is rendered from the
module's own skeleton. There is no copied schema and no cross-repo copy: the tests read the
module in place through `SPEC_OBJECTS_SECURITY_DIR`. Existing records are never opened for
writing (`create_new`). The accounting puts each symbol in exactly one bucket. Refusals use the
CLI's coded `Diagnostic` and `Outcome::Refused` shape, matching `anomaly.rs` and `update.rs`.
The help catalogue, the unknown-command list and the retained-help fixture edits agree with each
other and the tests pass. No ceremony was found.

## Verdict

**PASS WITH FINDINGS (three medium, two low).** The happy path is correct. Three edge paths break
the "write nothing" and "never write a second record" guarantees, or write an invalid record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | write_new writes records one at a time and returns Err on the first failure. Two failures abort part-way and leave the earlier records on disk with exit 1: a destination that holds a different document, and a file name the OS refuses (ENAMETOOLONG). Reproduced: pkg:cargo/a@1 was written, then a 252-char npm purl failed. Check every destination before creating any file, and make an occupied destination a coded refusal (exit 2) naming the file | rust/crates/quoin-cli/src/soup.rs:288-335 |
| FND-002 | medium | record_purl and read_ids each split frontmatter by hand (`strip_prefix("---\n")`), and silently skip any file over 1 MiB. A valid soup_record with CRLF line endings (quire validate exit 0) is invisible: promoting its purl wrote a second record for the same purl beside it (reproduced), which breaks FR-141 never-write-a-second-record and the duplicate refusal. Read frontmatter with quoin_quire::validate::frontmatter (the engine parser quoin-cli already links), and refuse an unreadable or oversized .md rather than ignore it | rust/crates/quoin-cli/src/soup.rs:272-283, rust/crates/quoin-cli/src/soup.rs:338-351 |
| FND-003 | medium | The percent-decoded purl name goes unescaped into the `# [id] title` heading line. An SBOM purl with `%0A` therefore injects Markdown into the record body. Reproduced: a name carrying `%0A%23%23 Properties...` wrote a record with two `## Properties` sections that quire validate refuses (duplicate-section, exit 3), and the command still exited 0. Refuse or skip a component whose decoded name or version contains a control character, or keep the heading single-line | rust/crates/quoin-cli/src/soup/plan.rs:214-245 |
| FND-004 | low | render drops any skeleton body line that starts with `\| supplier `. FR-141 asks only that the `supplier` frontmatter key be absent, and the Properties row is the type's field declaration, not a value. The filter couples the command to the skeleton's text and edits the module's declaration. Copy the skeleton body unchanged apart from the heading | rust/crates/quoin-cli/src/soup/plan.rs:244 |
| FND-005 | low | stated_obligations replaces a quire.coverage refusal with the prose Err "quire could not derive the obligations" (exit 1), so quire's own diagnostic is lost. `evidence record` returns the coverage response unchanged. Return the coverage Response as evidence.rs:116 does | rust/crates/quoin-cli/src/soup.rs:225-231 |

## Examined scope

```yaml
clean: false
scope:
- {id: soup.rs, path: rust/crates/quoin-cli/src/soup.rs, role: examined, excerpt: "run_promote, owning_skeleton (catalog.load), stated_obligations (quire.coverage), read_records, record_purl, write_new (create_new), read_ids, report"}
- {id: soup/plan.rs, path: rust/crates/quoin-cli/src/soup/plan.rs, role: examined, excerpt: "parse_purl, percent_decode, file_name, duplicate_records, plan, next_id_number, render, body"}
- {id: help.rs, path: rust/crates/quoin-cli/src/help.rs, role: examined, excerpt: "(\"soup\", crate::soup::ABOUT) and the unknown-command list gains soup"}
- {id: main.rs, path: rust/crates/quoin-cli/src/main.rs, role: examined, excerpt: "mod soup; dispatch soup::run; .subcommand(soup::command())"}
- {id: tests.rs, path: rust/crates/quoin-cli/src/tests.rs, role: examined, excerpt: "NATIVE_OWNED_ROUTES gains quoin soup and quoin soup promote; tc_1650 and tc_373 lists gain soup"}
- {id: retained-command-help.json, path: rust/crates/quoin-cli/tests/fixtures/retained-command-help.json, role: examined, excerpt: "root help COMMANDS gains soup; two unknown-command stderr lists gain soup"}
- {id: tc_1515_soup_promote.rs, path: rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs, role: examined, excerpt: "11 subprocess tests over FR-141-AC-1..13; module read in place via SPEC_OBJECTS_SECURITY_DIR"}
```
