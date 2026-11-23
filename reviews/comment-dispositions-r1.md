<!-- reviewer-dispositions repo=agent-ix/quoin visibility=public quoin=0.36.8 module=spec-artifacts-process@v0.36.0 id=SR-17096 round=1 model=claude-opus-5-5 run=bd453c76-7802-4059-9c9d-b6621a2a6091 -->

Fix commit ff50c6a0. Not mergeable: new findings FND-008, FND-009 (medium) go back to the coder.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |

New findings (disposition pass 1):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-008 | medium | ambiguous | AC-27 commits only DOS-REL.md but AC-28 needs NOTES.md in that commit; README.md never placed |
| FND-009 | medium | ambiguous | AC-28 does not say each further commit is applied alone; stacked, the deletion case is vacuous |
| FND-010 | low | other | AC-27 export command omits --output |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'In the repository of FR-191-AC-27, with a non-selected file `dossier/NOTES.md` committed in the dossier commit and `export` run with exactly the inputs of the passing run, each of two further commits makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the output file unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'In the repository of FR-191-AC-27, with a non-selected file `dossier/NOTES.md` committed in the dossier commit and `export` run with exactly the inputs of the passing run, each of two further commits makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the output file unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
- fnd: FND-003
  outcome: fixed
  after_excerpt: One composed test builds a repository with a baseline commit, a Baseline file at it, and a release commit that is the single child of the baseline and adds only the EA Release document and the release snapshot file; signs and tags the release commit as in EA FR-047-AC-1 (SSH key mapped to `decision_owner`, record trailers present); runs `engineering-assurance release-verify` on the tag and obtains a `released` result; runs `quoin dossier generate --doc release-record --version V --snapshot <snapshot> --release-verify <result>` so `DOS-REL` is complete; commits only `dossier/DOS-REL.md`; and...
- fnd: FND-004
  outcome: fixed
  after_excerpt: The export exits 0, and the extracted PDF text holds the Release field `<TAG> (tag commit <full hash of the tagged commit>)` of quire-cli FR-034, and the Source commit and Date of the dossier commit.
- fnd: FND-005
  outcome: fixed
  after_excerpt: The release tag exists before the `DOS-REL` document does, because the EA `release-verify` result that `DOS-REL` needs ([FR-201](./FR-201-dossier-release-record-content.md)) needs the tag; the resolved `HEAD` at export is therefore a descendant of the tagged commit. `export` SHALL leave acceptance of `--release` to `quire export`, which accepts a tag other than one on `HEAD` only under the first-parent descendant rule of quire-cli FR-034 section A, and SHALL report its refusal as exit 4 with the report carrying `quire` exit code 3. When the dossier directory is the repository root, `quire e...
- fnd: FND-006
  outcome: fixed
  after_excerpt: '### Release chain


    - The release tag exists before the `DOS-REL` document does, because the EA `release-verify` result that `DOS-REL` needs ([FR-201](./FR-201-dossier-release-record-content.md)) needs the tag; the resolved `HEAD` at export is therefore a descendant of the tagged commit. `export` SHALL leave acceptance of `--release` to `quire export`, which accepts a tag other than one on `HEAD` only under the first-parent descendant rule of quire-cli FR-034 section A, and SHALL report its refusal as exit 4 with the report carrying `quire` exit code 3. When the dossier directory is the repo...'
- fnd: FND-007
  outcome: fixed
  after_excerpt: '### Release chain


    - The release tag exists before the `DOS-REL` document does, because the EA `release-verify` result that `DOS-REL` needs ([FR-201](./FR-201-dossier-release-record-content.md)) needs the tag; the resolved `HEAD` at export is therefore a descendant of the tagged commit. `export` SHALL leave acceptance of `--release` to `quire export`, which accepts a tag other than one on `HEAD` only under the first-parent descendant rule of quire-cli FR-034 section A, and SHALL report its refusal as exit 4 with the report carrying `quire` exit code 3. When the dossier directory is the repo...'
new_findings:
- fnd: FND-008
  severity: medium
  confidence: high
  method: spec-review/base
  check: ambiguous
  artifact_id: FR-191-AC-28
  related_ids:
  - FR-191-AC-27
  path: spec/functional/FR-191-dossier-command.md
  lines: '166'
  excerpt: 'In the repository of FR-191-AC-27, with a non-selected file `dossier/NOTES.md` committed in the dossier commit and `export` run with exactly the inputs of the passing run, each of two further commits makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the output file unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
  finding: AC-27 'commits only dossier/DOS-REL.md' while AC-28, set in the repository of AC-27, needs dossier/NOTES.md committed in the dossier commit; AC-28 also modifies README.md, which AC-27 never places in the repository. One fixture must hold both.
- fnd: FND-009
  severity: medium
  confidence: medium
  method: spec-review/base
  check: ambiguous
  artifact_id: FR-191-AC-28
  related_ids: []
  path: spec/functional/FR-191-dossier-command.md
  lines: '166'
  excerpt: 'In the repository of FR-191-AC-27, with a non-selected file `dossier/NOTES.md` committed in the dossier commit and `export` run with exactly the inputs of the passing run, each of two further commits makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the output file unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
  finding: AC-28 does not say each further commit is applied alone on the dossier commit; stacked, the NOTES.md deletion check passes only because the README.md commit is already in the chain.
- fnd: FND-010
  severity: low
  confidence: high
  method: spec-review/base
  check: other
  artifact_id: FR-191-AC-27
  related_ids:
  - FR-191-AC-28
  path: spec/functional/FR-191-dossier-command.md
  lines: '165'
  excerpt: One composed test builds a repository with a baseline commit, a Baseline file at it, and a release commit that is the single child of the baseline and adds only the EA Release document and the release snapshot file; signs and tags the release commit as in EA FR-047-AC-1 (SSH key mapped to `decision_owner`, record trailers present); runs `engineering-assurance release-verify` on the tag and obtains a `released` result; runs `quoin dossier generate --doc release-record --version V --snapshot <snapshot> --release-verify <result>` so `DOS-REL` is complete; commits only `dossier/DOS-REL.md`; and...
  finding: The AC-27 export command omits --output <PATH>, which export requires and AC-28's 'output file unchanged' refers to.
```

+++
