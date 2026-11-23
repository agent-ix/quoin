---
id: SR-20261123200829
title: "Code review of quoin PR 1031 (FR-141 SOUP promotion keeps the SBOM supplier and licence)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@9f5cf57a; rust/crates/quoin-evidence/src/adapters/{sbom,licence_policy}.rs, rust/crates/quoin-evidence/tests/tc_licence_policy.rs, rust/crates/quoin-soup/src/soup.rs, rust/crates/quoin-soup/src/soup/plan.rs, rust/crates/quoin-cli/tests/tc_1515_soup_promote.rs"
review_set: subset
---

# Code review of quoin PR 1031

## Summary

Ticket: PLAT-1812. This review covers the FR-041 reader change (supplier exposed, unreadable licence entries carry no text), the FR-150 licence-policy message for unreadable entries, and the FR-141 `quoin soup promote` change that writes `supplier` and `license`, reports field differences, conflicts and omissions, and now reads through `read_components`. The Rust lane (repo rust-style plus the ecosystem rust-review checklist) is folded in. Diff: `git diff origin/main...HEAD` at 9f5cf57a.

Gates run: `cargo fmt --all --check` passed. Focused tests passed: `tc_1515_soup_promote` 23/23, `tc_licence_policy` 16/16, quoin-evidence lib 75/75, quoin-soup lib 57 passed and 1 ignored (an external OSV report). `SPEC_OBJECTS_SECURITY_DIR` pointed at a spec-objects-security checkout. `cargo clippy -p quoin-soup -p quoin-evidence --all-targets -D warnings` could not reach the touched files. It stops on 13 errors in untouched quoin-evidence files (`qualify/runs.rs`, `store/gc.rs`, `store/records.rs`, `timestamp.rs`), which the brief puts out of scope. Clippy on the changed code is therefore unmeasured.

## Verdict

**PASS WITH FINDINGS: no high, one medium, three low.** The medium is a spec deviation on one edge: a supplier with a control character is treated as "states none" when an existing record is compared.

What is right:

- The single-readable-licence rule is correct. A licence that is unreadable, or sits beside an unreadable one, gives no `license` and is listed with the placeholder `unreadable`. A control-character licence is omitted and listed.
- The supplier rules match FR-041-AC-11 for absent, empty, whitespace-only, `NOASSERTION` and `NONE` values, and the supplier is kept as written.
- Created records never carry `license_fact_ref`. Duplicates take the first occurrence, and conflicts are listed per field.
- Existing records are never written, including one already at `--out` (`write_new` skips adopted purls). Differences are reported in both directions, and the `license_fact_ref` exemption holds.
- FR-150 precedence is denied, then not-allowed, then unknown. An unreadable entry adds the fixed text once, after the unlisted texts.
- `soup promote` now reads through `read_components`. `parse_sbom` is the same reader mapped to symbols, so the symbol list, order and merge accounting are unchanged and there is no second reader (CON-1).
- Every report list is sorted, so the report is deterministic.
- `DeclaredLicence.text: Option<String>` has no consumer outside quoin-evidence and quoin-soup in the workspace or the sibling repositories.
- The `tc_1515_soup_promote.rs` module lookup through a relative path is pre-existing. The PR adds callers of the same helper and does not make it worse.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | When an existing record is compared, a supplier with a control character is treated as no supplier. `state()` drops it to `None`, and `differences()` compares against that. FR-141 "Existing records" defines the stated supplier as "the exposed supplier, or none". Its "none" list (none, several, unreadable, control character) is attached to the licence only. The Behavior line "Where the SBOM states a supplier for a purl and the existing record states none, the command SHALL list the record and field" therefore applies. Scenario: an existing record with no `supplier`, and an SBOM supplier `Acme\u0007Corp`. The spec requires a supplier difference, but the code reports none and lists the field only under "omitted fields". No AC covers the case. Fix one side: compare the exposed supplier, or amend FR-141 to class a control-character supplier as none, as it does for the licence. | rust/crates/quoin-soup/src/soup/plan.rs:166-174,211-233 |
| FND-002 | low | `declared_supplier` applies the `NOASSERTION`/`NONE` rule to CycloneDX `supplier.name` as well. FR-041 says only "an SPDX value of `NOASSERTION` or `NONE`" is exposed as no supplier. Scenario: a CycloneDX component with `{"supplier":{"name":"NONE"}}` exposes no supplier, so its promoted record has no `supplier` line, although the spec would keep it as written. Restrict the rule to SPDX, or state the CycloneDX case in FR-041. | rust/crates/quoin-evidence/src/adapters/sbom.rs:140-144,213,241-244 |
| FND-003 | low | The "multiple or unreadable licences" report lines print each text through `str::escape_debug`. That escapes more than control characters: `"`, `'`, `\` and grapheme-extending marks change too. For example, the name `Foo "Bar"` is listed as `Foo \"Bar\"`, and a combining accent becomes `\u{301}`, so the report does not show "the declared texts" (FR-141 Behavior). A text that itself contains `, ` is also ambiguous after the `, ` join. Escape only control characters. | rust/crates/quoin-soup/src/soup.rs:610-622 |
| FND-004 | low | `DeclaredLicence { kind, text: Option<String> }` documents the invariant "`None` exactly when the kind is `Unreadable`", but the type does not enforce it (rust-style api-design: use a validated type when it makes an invariant explicit). As a result, the licence-policy arm `LicenceKind::Unreadable => unreadable = true` is unreachable, `Id`/`Name`/`Expression` with `None` are representable, and plan.rs `state()` would write the text of an `Unreadable` entry that carried `Some`. An enum carrying the text per readable kind removes all three. | rust/crates/quoin-evidence/src/adapters/sbom.rs:33-53; rust/crates/quoin-evidence/src/adapters/licence_policy.rs:177-195; rust/crates/quoin-soup/src/soup/plan.rs:175-196 |

## Examined scope

```yaml
clean: false
scope:
- {id: FR-041-AC-11, path: spec/functional/FR-041-sbom-inventory-evidence.md, role: examined, excerpt: "supplier.name / SPDX supplier; empty, absent, NOASSERTION, NONE expose nothing; run entries byte-identical"}
- {id: FR-041-AC-12, path: spec/functional/FR-041-sbom-inventory-evidence.md, role: examined, excerpt: "unreadable entries expose kind unreadable with no text, in document order"}
- {id: FR-150-AC-17, path: spec/functional/FR-150-licence-and-ban-policy-findings.md, role: examined, excerpt: "unreadable licence entry message, after unlisted texts"}
- {id: FR-141-AC-18..25, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "supplier/licence writing, omission, differences, conflicts"}
- {id: FR-141-CON-1, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "no SBOM reader of its own"}
- {id: FR-141-CON-3, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "no parse, normalise, map or join of licence text"}
- {id: FR-141-CON-4, path: spec/functional/FR-141-promote-sbom-components-to-soup-records.md, role: examined, excerpt: "never overwrite a field of an existing record"}
```

## Dispositions

Round 1 was checked at 323a26f9 (fix commits 901d9254 and 323a26f9). Focused tests passed: `tc_1515_soup_promote` 26/26, `tc_licence_policy` 18/18, quoin-evidence lib 75/75, quoin-soup lib 57 passed and 1 ignored. `cargo fmt --all --check` passed. Clippy is still blocked by the pre-existing errors in untouched quoin-evidence files. Each fix was checked by a mutation that put back the pre-fix behaviour, run on a temporary working-tree edit that was then reverted:

- Dropping a control-character supplier from `Stated` again fails `a_supplier_with_a_control_character_is_omitted_but_still_compared`.
- Filtering CycloneDX `NONE`/`NOASSERTION` again fails `tc_1812_001_the_declared_supplier_is_exposed_as_written`.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `Stated.supplier` is now the exposed supplier, and `render` alone omits a control-character supplier, so `differences()` compares the exposed supplier as FR-141 states. The test `a_supplier_with_a_control_character_is_omitted_but_still_compared` asserts both the omission and the difference, and it fails under the pre-fix mutation. |
| FND-002 | fixed | CycloneDX `supplier.name` is filtered only for empty or whitespace-only values. `NOASSERTION`/`NONE` stay an SPDX-only rule, and `tc_1812_001` asserts that a CycloneDX `NONE` is exposed. The test fails under the pre-fix mutation. |
| FND-003 | fixed | Each declared text is printed as written on its own indented line. Only control characters are spelled out (`visible`), so quotes and backslashes pass through, and the `, ` join is gone. `a_declared_text_is_listed_as_written_on_its_own_line` asserts this. |
| FND-004 | fixed | `DeclaredLicence` is now an enum `Id(String) / Name(String) / Expression(String) / Unreadable`, and `LicenceKind` is removed. The licence policy and `state()` match on the variants, and no unreachable arm remains. No other consumer exists in the quoin workspace; the only users are quoin-evidence and quoin-soup, and both are updated. |
