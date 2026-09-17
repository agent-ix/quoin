# Semantic-module mapping fixtures (quoin FR-071, FR-072, FR-074, FR-104)

These fixtures are the normative surface of the Markdown → semantic-core mapping
that `agent-ix/quire-rs#388` and `agent-ix/quire-rs#418` implement. Quoin does not execute the extraction;
its tests assert that the table and fence forms of the same content share one
expected `FieldDecl[]` and that every expected diagnostic carries a locus. Each
file records its semantic-core version. The vendored schemas under
`src/semantic/schemas/semantic-core/` are `0.1.0`, so only the `0.1.0` expected
outputs validate against them in quoin today; the `0.2.0` outputs
(`operations.*`, `relationships.*`) validate in quoin only once agent-ix/quoin#549
vendors the `0.2.0` schemas.

| File | Requirement | Hand-off |
|---|---|---|
| `config-version.table.md`, `config-version.fence.md`, `config-version.expected.json` (semantic-core `0.1.0`) | FR-071-AC-1, AC-2, FR-072-AC-7 | Quire extracts both to the expected array (normalized) and carries the `ocl` clause with the listed advisory and lossy availability |
| `both-forms.md` | FR-071-AC-3 | #388 fails at the second form's locus |
| `cell-cases.json` | FR-071-AC-4..AC-8 | #388 executes each cell/fence-line case |
| `operations.md`, `operations.expected.json`, `operations-cases.json` (semantic-core `0.2.0`) | FR-072-AC-1..AC-7 | Quire extracts clauses/operations, emits the listed diagnostics, and reports the listed availability |
| `clause-language-0.1.0-cases.json` (semantic-core `0.1.0`) | FR-072-AC-8 | Quire refuses a `quire` fence under `0.1.0` at the fence |
| `relationships.md`, `relationships.expected.json`, `relationships-cases.json` (semantic-core `0.2.0`) | FR-104-AC-1..AC-10 | #418 extracts `RelationDecl[]` and `relationSources` (the `0.2.0` carrier until agent-ix/filament-core-data#155) under the recorded registry and bundle `context`, pinned to the spec-artifacts-iso and spec-objects-business revisions in `context.sources`, emits the listed diagnostics, and reports the listed availability |
| `legacy-bullets.md`, `legacy-mixed.md`, `legacy.expected.json`, `../corpus/config-service/` | FR-074-AC-1, AC-2 | quoin's `classifyArtifact` and #388 agree on form, line, and warning |

## `relationships-cases.json` fields

Each case in `cases[]` has these fields:

| Field | Meaning |
|---|---|
| `id` | Case name, cited by FR-104 acceptance criteria. |
| `relationships` | Body appended to `artifactHead`; body line 1 is artifact line 18. |
| `artifact` | A full artifact used verbatim, in place of `artifactHead` + `relationships`. |
| `mappings` | Replaces `context.mappings` for this case. |
| `withoutRelationVocabulary` | When `true`, the extraction runs with no relation vocabulary: no `edgeTypes`, `roles`, or `allowedLinks` (FR-104 `no-relation-vocabulary`). |
| `withoutBundleIndex` | When `true`, the extraction runs with no bundle index: no `bundle.artifacts`; `bundle.package` and `bundle.imports` stay (FR-104 `no-bundle-index`). |
| `diagnostics[]` | Expected diagnostics: `code`, `severity`, `locus`, `line`, `section`, `reason`, and optionally `messageContains`. |
| `diagnostics[].messageContains` | Substrings the diagnostic `message` must contain. |
| `exactDiagnostics` | The exact number of diagnostics the extraction emits; a row that fails several checks yields one (FR-104 refusal order). |
| `relations`, `relationSources` | Expected outputs; `relations: null` means neither is emitted. |
| `availability` | Expected `availability`; `{}` means no `availability.relations` key. |

`context.sources` records the repository, revision, and manifest path the registry and object-type facts come from.

`../corpus/config-service/FR-006-config-version-entity.md` is a verbatim copy
pinned by `PROVENANCE.json`; it is never edited.
