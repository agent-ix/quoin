---
id: SR-17096
title: "Base spec review of the FR-191 release chain amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quoin; spec/functional/FR-191-dossier-command.md (diff origin/main...HEAD, PR quoin#1037)"
review_set: subset
---

# Base spec review of the FR-191 release chain amendment

## Summary

Ticket: PLAT-1811. Reviewed the `Release chain` section, FR-191-AC-27, FR-191-AC-28 and the Dependencies line added to FR-191 by quoin#1037, against origin/main of quoin (FR-191 Commit then export, Export currency and exit table, FR-201, FR-096 exit taxonomy, `quoin-dossier` release record and the `quoin-cli` dossier export path), the merged quire-cli FR-034, FR-038 and FR-040, and EA FR-047 and FR-051.

The direction is consistent with every upstream: quire-cli now accepts a `--release` tag on a first-parent release ancestor whose later commits only add or modify files under an input's directory, EA release-verify needs the tag and requires the release commit to add only the record and snapshot, so the dossier must follow the tag. The exit wording matches the code: a `quire export` failure maps to `DossierErrorCode::QuireFailed`, whose outcome is 4 (FR-096 internal fault), with the report member `quire:{exit_code, stderr}`, and every quire-cli `--release` refusal is exit 3 (FR-040 Source commit unavailable row). AC ids 27 and 28 are new: no earlier revision of FR-191 on any ref held them and no `#[trace]` tag references them. `quire validate` passes the changed file (1/1 grammar-clean).

The composed test of FR-191-AC-27 is realistic: quoin already runs real engineering-assurance with real `git` and `ssh-keygen` signed tags (`tc_plat_1789_signed_approval_chain.rs`, `#[ignore]` plus `QUOIN_E2E_EA`, `make e2e-signing`, which fails rather than skips) and real `quire` (`tc_1614_dossier_command.rs`, ignored test). The order tag, release-verify, generate, dossier commit, export is possible. The defects are in the two ACs as written: the deletion case of AC-28 cannot reach `quire export` in the AC-27 repository, and AC-27 omits the inputs that its exit 0 needs and gives a weak Release oracle.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-28 deletion case cannot reach `quire export`: in the AC-27 repository the only file under `dossier/` is `DOS-REL.md` (`generate --doc release-record`), and deleting it makes the selected committed blob missing at HEAD, so FR-191 Export currency exits 1 before `quire export` starts (FR-191-AC-7), never exit 4 with quire code 3. The AC must delete a file under `dossier/` that is not a selected document (added for the purpose). | FR-191-AC-28 |
| FND-002 | medium | AC-28 modification case does not name the file: modifying a file that a generator input reads (the `--snapshot` file, a source record) changes the regenerated `DOS-REL` bytes, so export exits 1 on currency, not 4. The AC should name a file no source reads (for example a README) and say the export runs with the same inputs as the passing run. | FR-191-AC-28 |
| FND-003 | medium | AC-27 omits what its exit 0 needs. Export exits 0 only when `DOS-REL` is complete, which needs a `released` EA result (a tag signed per EA FR-047-AC-1 by the key the allowed-signers file maps to `decision_owner`, a Baseline file at the baseline commit, trailers binding the record), and `generate` needs `--snapshot` and `--version`; `export` must be given the same `--release-verify`, `--snapshot` and `--version` or regeneration differs and exits 1. "tags it" and "with that result" leave all of this to inference, and "release record" is ambiguous between the EA Release document and `DOS-REL`. | FR-191-AC-27 |
| FND-004 | medium | AC-27 Release oracle is weak: the tag commit differs from the source commit, so quire-cli FR-034 shows Release as `<TAG> (tag commit <full hash of the release commit>)`. "holds the tag as the Release field" passes on a substring match even if the tag commit hash is missing or wrong. State the exact Release text and that Source commit and Date are those of the dossier commit. | FR-191-AC-27 |
| FND-005 | low | Release chain bullet 2 restates the quire rule as sufficient ("satisfies ... when the commits ... add or modify only files under the exported dossier's directory"), which is weaker than quire-cli FR-034 section A (first-parent chain, one parent each, regular files with unchanged mode, no symlink or submodule, no root input). A merge or mode change touching only `dossier/` is refused by quire. Say "only when", or cite the release-ancestor rule without restating it; note that `--out` at the repository root admits only a tag on HEAD. | FR-191 |
| FND-006 | low | Release chain bullet 1 repeats the existing Commit then export SHALL that `export` passes `--release` unchanged, and joins to it a workflow clause ("the dossier is committed after it") that is not `export` behaviour; quoin never commits. | FR-191 |
| FND-007 | low | Release chain bullet 3 ("The repository SHALL hold a test ...") is a requirement on the test suite that duplicates FR-191-AC-27; the AC alone carries it. | FR-191 |

## Verdict

Not mergeable as is: FND-001 makes one AC-28 case unsatisfiable as written, and FND-002 to FND-004 leave the composed test underspecified. All are small wording fixes inside FR-191. No migration, compatibility or fallback language, no pins, ledgers or hashes, no paywalled text and no machine paths were found. Consistent and clean: the exit-4 / quire-code-3 wording, the order of tag, release-verify and dossier commit, the `--out` reading (the input directory is `<DIR>`, so FR-191-AC-10 `--out other` stays consistent), the DOS-REL Decision text at `release_record.rs` (which claims no binding to the DOS-REL bytes, so a dossier committed after the tag contradicts nothing), and the Dependencies line.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-191-AC-27 and FR-191-AC-28 disagree on the fixture: AC-27 "commits only `dossier/DOS-REL.md`", while AC-28, set "in the repository of FR-191-AC-27", requires `dossier/NOTES.md` "committed in the dossier commit"; AC-28 also modifies `README.md`, which AC-27 never places in the repository. One fixture must hold both: the AC-27 dossier commit adds `DOS-REL.md` and `NOTES.md` under `dossier/`, and the baseline holds `README.md`. | FR-191-AC-28 |
| FND-009 | medium | FR-191-AC-28 does not say that each of the two further commits is applied alone on the dossier commit. If they are stacked, the `NOTES.md` deletion check passes only because the `README.md` commit is already in the chain, so the deletion refusal is never shown on its own. | FR-191-AC-28 |
| FND-010 | low | The FR-191-AC-27 export command omits `--output <PATH>`, which `export` requires and which AC-28's "output file unchanged" refers to. | FR-191-AC-27 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | AC-28 now deletes the non-selected `dossier/NOTES.md`; export currency checks only selected documents (`<DIR>/<document-id>.md`), so it passes and `quire export` refuses the deletion. |
| FND-002 | fixed | AC-28 names `README.md`, read by no source record or snapshot, and runs export with the inputs of the passing run. |
| FND-003 | fixed | AC-27 states the baseline, Baseline file, single-child release commit, FR-047-AC-1 signed tag, `released` result, and the same `--version`, `--snapshot`, `--release-verify` on generate and export; "EA Release document" and `DOS-REL` are distinct. |
| FND-004 | fixed | AC-27 requires Release `<TAG> (tag commit <full hash of the tagged commit>)` and the Source commit and Date of the dossier commit. |
| FND-005 | fixed | Bullet cites the first-parent descendant rule of quire-cli FR-034 section A as the only acceptance path and adds the repository-root case. |
| FND-006 | fixed | Duplicate `--release` bullet and workflow clause removed. |
| FND-007 | fixed | Test-existence bullet removed. |
