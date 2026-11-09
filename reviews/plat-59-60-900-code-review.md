---
id: SR-16329
title: "Code review of quoin PR 774, ocl to quire fences in templates and fixtures"
type: SpecReview
analysis: code-review
scope: "agent-ix/quoin@93b3504; templates/semantic-module/**; tests/fixtures/semantic-module/mapping/**; spec/functional/FR-070-semantic-module-manifest-extension.md; spec/functional/FR-072-invariants-and-operations-mapping.md; spec/functional/FR-079-generated-skeletons-and-fixtures.md; spec/matrix.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quoin/FR-079
    type: reviews
  - target: ix://agent-ix/quoin/FR-072
    type: reviews
---

# Code review of quoin PR 774

## Summary

Ticket: PLAT-59, PLAT-60, PLAT-900. Frozen-head review of quoin#774 at 93b3504, scoped to
`git diff origin/main...HEAD` (20 files). No Rust or React in the diff, so neither lane applies.
Tools: quoin 0.29.1, quire 0.36.2 (engine 0.50.2).

Checked against the engine, not only the PR: in quire-rs origin/main, `clause_language_class`
(`src/semantic/clauses.rs:177`) returns `Checked` for exactly `quire`, which emits no
diagnostic and leaves the kind non-lossy. `ocl`/`sysml`/`fretish` are `Unchecked`: advisory plus
lossy. quire-rs FR-071 says the engine SHALL NOT check clause text. So the golden's
`language: quire`, `lossy: false`, `diagnostics: []` is what the extraction emits today.

Measured: all three changed JSON files parse. The template renders with `cookiecutter --no-input`,
and the rendered output holds no `ocl`. In the rendered repo,
`test_every_skeleton_carries_a_quire_clause_under_its_own_heading` passes (13 passed; the
2 errors are setup failures that need emitted schemas from npm). The rename to `quire-clause` is
consistent across manifest.yaml (`mappings`), mappings.yaml (`kinds` and both `clauses.kind`)
and the records. No digests, pins or other ceremony are added. The `ocl` mentions that remain in
tracked files are legitimate: the FR-072 unchecked-language behaviour, `fence-ocl-carried`,
completed plan history, the frozen Jev eval corpus, and an unrelated relationships fixture.

Not run: the quoin full gate and anything that needs quoin-semantic's node_modules (the npm.ix
registry is down).

## Verdict

**FAIL: one high finding.** The tag swap is mechanical and consistent. But every relabelled
fence still carries OCL text (`context X inv Y:`, `->notEmpty()`, `@pre`, `<>`). The template now
declares OCL as the checked, lossless `quire` clause language. Separately, a fixture the PR left in
place asserts the opposite of the new golden.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Every `quire`-tagged fence still holds OCL syntax (`context Shipment inv X:`, `->notEmpty()`, `@pre`, `<>`), which is not Quire clause language (QSL uses `pre(e)`, `implies`, and no `context/inv` header). The mapping (`lossless: true`), the records and the golden (`lossy: false`) declare these clauses as checked. FR-072-CON-1 hands `quire` clauses to the Quire checker, which would refuse them. PLAT-59 asks for invariants written in quire, not only retagged | templates/semantic-module/{{cookiecutter.repo_name}}/{{cookiecutter.package_name}}/skeletons/element.md:38-49; skeletons/note.md:33-36; skeletons/*.sysml.md; tests/fixtures/negative/note-missing-properties-section.md:14; tests/fixtures/semantic-module/mapping/config-version.table.md:34-36; config-version.fence.md:34-36; config-version.expected.json:113 |
| FND-002 | medium | `clause-language-0.1.0-cases.json` (case `fence-quire-invalid`) and README row 14 still assert that Quire refuses a `quire` fence with `semantic.clause-language-invalid`. That contradicts the new golden and the engine, which always classifies `quire` as Checked, and it covers withdrawn FR-072-AC-8. Delete the file and the row | tests/fixtures/semantic-module/mapping/clause-language-0.1.0-cases.json:1-25; tests/fixtures/semantic-module/mapping/README.md:14 |
| FND-003 | low | The README the PR edits still points at `../corpus/config-service/` ("a verbatim copy; it is never edited") in the legacy row and its last line. That directory no longer exists (de-vendored, PLAT-887) | tests/fixtures/semantic-module/mapping/README.md:15; tests/fixtures/semantic-module/mapping/README.md:45 |

## Examined scope

```yaml
clean: false
scope:
- {id: skeletons/element.md, path: "templates/semantic-module/{{cookiecutter.repo_name}}/{{cookiecutter.package_name}}/skeletons/element.md", role: examined, excerpt: "```quire\ncontext Shipment\ninv WeightIsPositive:\n  self.weight_grams > 0"}
- {id: mappings.yaml, path: "templates/semantic-module/{{cookiecutter.repo_name}}/{{cookiecutter.package_name}}/mappings.yaml", role: examined, excerpt: "kind: quire-clause ... language: quire ... lossless: true"}
- {id: manifest.yaml, path: "templates/semantic-module/{{cookiecutter.repo_name}}/{{cookiecutter.package_name}}/manifest.yaml", role: examined, excerpt: "mappings: [typed-table, sysml-fence, quire-clause]"}
- {id: config-version.expected.json, path: tests/fixtures/semantic-module/mapping/config-version.expected.json, role: examined, excerpt: "\"language\": \"quire\" ... \"lossy\": false ... \"diagnostics\": []"}
- {id: test_skeletons_semantic.py, path: "templates/semantic-module/{{cookiecutter.repo_name}}/tests/test_skeletons_semantic.py", role: examined, excerpt: "r\"^### (\\w+)\\n\\n```quire\\n(.*?)```\""}
- {id: quire-rs clause_language_class, path: "quire-rs:src/semantic/clauses.rs", role: context_only, excerpt: "if tag == \"quire\" { return ClauseLanguageClass::Checked; } if matches!(tag, \"ocl\" | \"sysml\" | \"fretish\") { return ClauseLanguageClass::Unchecked; }"}
```

## Dispositions

Round 1, checked against fix commit d53395c6 (head 0cd7173). The reviewer ran the installed quire Python
wheel 0.53.0, `quire.extract_semantic`, on all four skeletons, the negative fixture and both
config-version fixtures. Every clause extracts as `language: quire` with no clause diagnostic and
`clauses: {state: available, lossy: false}`. The config-version table extraction equals the golden's
`clauses` and `clauseText`. Element and Note records equal the extracted `clauses`, spans included.
The quire CLI 0.36.2 has no surface that type-checks a Quire clause body, and no QSL checker binary is
built on this machine. Grammar validity was therefore judged against quire-spec-language origin/main:
the `qsl-cst` tokens `present`, `value`, `!=` and `pre`, plus FR-056 and FR-104.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | Skeletons and negative fixture fixed: `present(e) implies ...`, `self.weight_grams > 0` and `value(e) != ""` are QSL value forms, and the fields they read exist. The config-version `immutable` fence is still not valid Quire where it is used. It is a standalone `## Invariants` clause, no `Post:` line binds it (the fixture has no `## Operations`), and its body reads `pre(...)`. quire-spec-language FR-104 admits `pre(e)` only in a post clause. Fix: in config-version.table.md and config-version.fence.md, replace the clause with a current-state invariant, for example `### versionIsPositive` / `self.versionNumber >= 1`. Update the golden's `clauses[0].clauseId` and `clauseText` to match; the spans do not move. The coder's comparison with operations.md `archived` does not hold, because `Post: archived` binds that clause as a postcondition |
| FND-002 | fixed | clause-language-0.1.0-cases.json deleted; README row 14 removed |
| FND-003 | fixed | Both `../corpus/config-service/` references removed from the README |
