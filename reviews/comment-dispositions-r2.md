<!-- reviewer-dispositions repo=agent-ix/quoin visibility=public quoin=0.36.8 module=spec-artifacts-process@v0.36.0 id=SR-17096 round=2 model=claude-opus-5-5 run=bd453c76-7802-4059-9c9d-b6621a2a6091 -->

Fix commit b1382836. FND-008..010 fixed; one new low finding (FND-011), non-blocking.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-008 | fixed | - |
| FND-009 | fixed | - |
| FND-010 | fixed | - |

New findings (disposition pass 2):

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-011 | low | ambiguous | AC-28 reads the --json report but says "exactly the inputs of the passing run", which has no --json |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-008
  outcome: fixed
  after_excerpt: One composed test builds a repository with a baseline commit that holds a Baseline file and a `README.md`, and a release commit that is the single child of the baseline and adds only the EA Release document and the release snapshot file; signs and tags the release commit as in EA FR-047-AC-1 (SSH key mapped to `decision_owner`, record trailers present); runs `engineering-assurance release-verify` on the tag and obtains a `released` result; commits only `dossier/DOS-REL.md` and a non-selected `dossier/NOTES.md` (the dossier commit)
- fnd: FND-009
  outcome: fixed
  after_excerpt: 'In the repository of FR-191-AC-27, with `export` run with exactly the inputs of the passing run, including `--output <PATH>`, each of two further commits, applied on its own on top of the dossier commit as separate cases, makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the file at `<PATH>` unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
- fnd: FND-010
  outcome: fixed
  after_excerpt: and runs `quoin dossier export --doc release-record --version V --snapshot <snapshot> --release-verify <result> --release <TAG> --to pdf --output <PATH>` with a real `quire export`. The export exits 0, and the extracted PDF text holds the Release field `<TAG> (tag commit <full hash of the tagged commit>)` of quire-cli FR-034, and the Source commit and Date of the dossier commit.
new_findings:
- fnd: FND-011
  severity: low
  confidence: medium
  method: spec-review/base
  check: ambiguous
  artifact_id: FR-191-AC-28
  related_ids:
  - FR-191-AC-27
  path: spec/functional/FR-191-dossier-command.md
  lines: '166'
  excerpt: 'In the repository of FR-191-AC-27, with `export` run with exactly the inputs of the passing run, including `--output <PATH>`, each of two further commits, applied on its own on top of the dossier commit as separate cases, makes `export` exit 4 with the report carrying `quire` exit code 3 and the `quire export` refusal in its `stderr` member, and leaves the file at `<PATH>` unchanged: a commit that modifies `README.md` outside `dossier/` (a file no source record or snapshot reads), and a commit that deletes `dossier/NOTES.md`.'
  finding: AC-28 runs export 'with exactly the inputs of the passing run' and reads quire exit code and stderr from the report, but the AC-27 passing run has no --json and export emits the report only with --json (FR-191 Outputs; quoin-cli dossier report_response). Name --json as the one added flag.
```

+++
