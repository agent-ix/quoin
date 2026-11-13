---
id: SR-20261113000907
title: "Gap analysis of quoin PR 838 (FR-141 coverage)"
type: SpecReview
analysis: base
scope: "agent-ix/quoin@e4fa4c89; spec/functional/FR-141-promote-sbom-components-to-soup-records.md, rust/crates/quoin-cli/src/soup.rs, rust/crates/quoin-cli/src/soup/plan.rs, rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs"
review_set: subset
---

# Gap analysis of quoin PR 838

## Summary

Ticket: PLAT-1515. Plan completion: not assessed. Measured against FR-141 at e4fa4c89, with
FR-041, FR-046 and FR-133 as context, and against agent-ix/spec-objects-security
FR-009..FR-012 at origin/main (#45, cloned into the scratchpad). `quire trace --id
FR-141-AC-<n>` (quire 0.44.0, engine 0.59.1) found at least one `#[trace]` claim for every
criterion AC-1..AC-13. The 11 integration tests and 4 unit tests pass. Each binding was read
to check that its test can fail, and all 13 assert observable behaviour: file counts, exact
report lines, quire validation, byte snapshots, exit codes and stderr naming. CON-1 holds by
inspection: the only reader is `quoin_evidence::adapters::parse_sbom`, and `parse_purl` reads
the adapter's symbol, not the document. CON-2 holds: the schema and skeleton come from the
installed module through `catalog.load`.

The PLAT-1515 ticket criterion "record count equals component count" is superseded by FR-141
(FR-141 Description and the ticket comment). The FR-141-AC-2 accounting is what was checked.

The author reported ten spec decisions. Each one is resolved below. FND-001 and FND-002 carry
the amendment text, and the rest need no spec change.

| Gap | Disposition | Recommended FR-141 text |
| --- | --- | --- |
| 1 id scheme | amend + tag tc_1515_004 | Behavior: "The command SHALL give each new record the id `SOUP-<n>`, `<n>` at least three digits, numbering on from the highest `SOUP-` id among the records in the `--records` and `--out` directories, in ascending purl order." |
| 2 hex case | amend | "... replaced by `%` and its two uppercase hex digits ..." |
| 3 occupied out-file | amend + code change (code-review FND-001) | "If the file a new record would be written to already exists, then the command SHALL count it unchanged when it is the `soup_record` of the same purl, and otherwise SHALL exit non-zero naming the file and write nothing." |
| 4 decoded segments | amend | Description: "A record's `name` and `version` are the percent-decoded name and version segments of its purl." Add: "If a decoded name or version contains a control character, then the command SHALL skip the component and list it in the report." (code-review FND-003) |
| 5 superseded scope | amend | "... equals that of an existing record in the records directory whose version segment differs as written ..." |
| 6 --module | amend | "Where `--module <dir>` is given, the command SHALL look up the owning module only in the given module roots." |
| 7 single id, text, file | no change | FR-141 already names one `<obligation-id>`. A deterministic text report meets the report statement, and stdin input is not promised |
| 8 skipped naming | amend (clarification) | "... SHALL list it in the report by its FR-041 symbol (`name@version`, or `name` where no version is stated)." |
| 9 name/version not engine-validated | no FR-141 change | AC-1 asserting the frontmatter directly is correct. The gap belongs to spec-objects-security: FR-009 says the schema admits `name`/`version`/`supplier`, but `body_extraction` yields only id, title, type, purl and properties, so those keys are never schema-checked. Raise it with that repo's owner |
| 10 programmatic fixture | no change | Extending the real fixture inside the test copies nothing and still meets AC-1's "a CycloneDX and an SPDX fixture" |

## Verdict

**PASS WITH FINDINGS (one medium, four low).** Every criterion is bound by a test that can fail.
The medium finding is a FR-141 file-naming rule that fails on realistic purls.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-141 names a record file by its whole purl with every byte outside `A-Za-z0-9._-` percent-encoded. Qualifiers make that exceed the 255-byte file-name limit. Syft npm purls carry `download_url` (the repo's own cyclonedx-real.json purl yields a 110-byte name), and a 252-char scoped-npm purl failed with ENAMETOOLONG (reproduced). Amend: "The command SHALL name each new record file by its purl without qualifiers and subpath, with every byte outside `A-Za-z0-9._-` replaced by `%` and its two uppercase hex digits, followed by `.md`." A second purl that maps to an occupied name then falls under the gap-3 rule | spec/functional/FR-141-promote-sbom-components-to-soup-records.md:53 |
| FND-002 | low | FR-141 leaves unstated behaviour that the code decided: the record id scheme, hex case, an occupied out-file, decoded segments, the superseded scope, `--module` and the skipped symbol form (author gaps 1-6 and 8). Add the amendment text in the Summary table in this PR. tc_1515_004 then gets a `#[trace]` to the new id criterion | spec/functional/FR-141-promote-sbom-components-to-soup-records.md:36-60 |
| FND-003 | low | The FR-141-AC-4 clause "without it no `disposition_clause` key" is never exercised with `--discharges` absent. The absence test passes `--discharges OB-NOPE`, which is AC-10's path. Assert that no `disposition_clause` key is written in a run without the flag, for example in the AC-1 test | rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs:228-259 |
| FND-004 | low | In the AC-13 test, `!n.contains('/')` is vacuous: a name returned by read_dir can never contain `/`. The binding holds only through the exact-name assertion `pkg%3Acargo%2Fserde%401.0.228.md` and the distinct-count check. Drop the vacuous clause or assert on `file_name()` output directly | rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs:460-465 |
| FND-005 | low | The doc comment on the production fn `plan` names `FR-141-AC-2`, and `quire trace` reports it as a production_orphan_tag citation. quoin rust-style says an id in a comment binds nothing. Drop the id from the comment | rust/crates/quoin-cli/src/soup/plan.rs:143 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-141-AC-1, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "A CycloneDX and an SPDX fixture that list the same three components with purls each promote to three `soup_record` files that validate against the owning schema, each carrying `name` and `version` equal to its purl's segments and no `supplier`."}
- {id: FR-141-AC-2, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "For a fixture with purl-bearing components, a repeated purl and a component with no purl, the report states components read, records created, records unchanged, merged duplicates and components skipped, and created + unchanged + merged + skipped equals components read."}
- {id: FR-141-AC-3, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "A component with no purl writes no record and appears by name and version in the report; a component with neither purl nor name is dropped by the FR-041 adapter and is not reported as skipped."}
- {id: FR-141-AC-4, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "A newly created record carries none of `serves`, `known_anomalies`, `support_status`, `end_of_support`, `supplier`; with `--discharges OB-1` naming a stated obligation it carries `disposition_clause: OB-1`, and without it no `disposition_clause` key."}
- {id: FR-141-AC-5, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "Given an existing record whose reviewer-authored fields are populated, promoting the same document leaves the file byte-identical and the report lists it as unchanged."}
- {id: FR-141-AC-6, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "After promoting a document whose component version was bumped, a new record exists for the new purl, the old record is byte-identical, and the report lists the old record as a superseded candidate."}
- {id: FR-141-AC-7, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "Promoting the same document twice leaves the `--out` and `--records` directories byte-identical after the second run."}
- {id: FR-141-AC-8, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "The command with the owning module absent exits non-zero naming the module and creates no file."}
- {id: FR-141-AC-9, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "A document that is neither CycloneDX nor SPDX is refused with the FR-041-AC-4 error and writes no record."}
- {id: FR-141-AC-10, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "A `--discharges` id no obligation states is reported `unmatched`, and the written records carry no `disposition_clause`."}
- {id: FR-141-AC-11, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "With `--records` naming a directory that already holds a record for a purl in the document and `--out` naming another directory, no second record for that purl is written."}
- {id: FR-141-AC-12, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "Two records in the records directory carrying one purl make the command exit non-zero naming both files and write nothing."}
- {id: FR-141-AC-13, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "With no `--out`, new records land in `spec/soup/` of the repository, and the file name of a purl containing `/` and `@` contains no `/` and is distinct from the file name of every other purl in the fixture."}
- {id: FR-141-CON-1, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "The command SHALL NOT contain an SBOM reader of its own; the reading is the FR-041 adapter's."}
- {id: FR-141-CON-2, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "The command SHALL NOT define a record type; the schema is the owning module's."}
- {id: FR-041-AC-8, path: spec/functional/FR-041-sbom-inventory-evidence.md, role: context_only, excerpt: "A `--discharges` id that no obligation states is reported `unmatched` and binds nothing, including when the inventory lists no component."}
- {id: FR-009, path: "agent-ix/spec-objects-security:spec/functional/FR-009-soup-record-identity-and-promotion.md", role: context_only, excerpt: "A SOUP record identifies one third-party component by its purl"}
- {id: FR-011, path: "agent-ix/spec-objects-security:spec/functional/FR-011-soup-risk-disposition.md", role: context_only, excerpt: "the promotion fills `disposition_clause` from the obligation id of the `--discharges` binding"}
bindings:
- {test_id: a_cyclonedx_and_an_spdx_inventory_promote_to_valid_records_without_a_supplier, ac_id: FR-141-AC-1, trace: correct}
- {test_id: tc_1515_001_name_and_version_are_the_purl_segments, ac_id: FR-141-AC-1, trace: correct}
- {test_id: the_report_accounts_for_every_component_and_names_a_skipped_one, ac_id: FR-141-AC-2, trace: correct}
- {test_id: tc_1515_003_every_component_is_counted_once, ac_id: FR-141-AC-2, trace: correct}
- {test_id: the_report_accounts_for_every_component_and_names_a_skipped_one, ac_id: FR-141-AC-3, trace: correct}
- {test_id: a_new_record_carries_no_reviewer_field_and_an_unstated_discharge_is_unmatched, ac_id: FR-141-AC-4, trace: correct}
- {test_id: a_stated_obligation_is_written_verbatim_as_the_disposition_clause, ac_id: FR-141-AC-4, trace: correct}
- {test_id: promoting_again_changes_no_file_and_never_touches_a_reviewed_record, ac_id: FR-141-AC-5, trace: correct}
- {test_id: a_bumped_version_gets_a_new_record_and_reports_the_old_one_as_superseded, ac_id: FR-141-AC-6, trace: correct}
- {test_id: tc_1515_003_every_component_is_counted_once, ac_id: FR-141-AC-6, trace: correct}
- {test_id: promoting_again_changes_no_file_and_never_touches_a_reviewed_record, ac_id: FR-141-AC-7, trace: correct}
- {test_id: without_the_owning_module_the_command_names_it_and_writes_nothing, ac_id: FR-141-AC-8, trace: correct}
- {test_id: a_document_that_is_not_an_sbom_is_refused_and_writes_nothing, ac_id: FR-141-AC-9, trace: correct}
- {test_id: a_new_record_carries_no_reviewer_field_and_an_unstated_discharge_is_unmatched, ac_id: FR-141-AC-10, trace: correct}
- {test_id: a_record_in_another_records_directory_stops_a_second_one_being_written, ac_id: FR-141-AC-11, trace: correct}
- {test_id: two_records_with_one_purl_stop_the_command_naming_both_files, ac_id: FR-141-AC-12, trace: correct}
- {test_id: with_no_out_records_land_in_spec_soup_under_separator_free_names, ac_id: FR-141-AC-13, trace: correct}
- {test_id: tc_1515_002_file_names_hold_no_separator_and_stay_distinct, ac_id: FR-141-AC-13, trace: correct}
```
