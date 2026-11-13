# Positive installed-pack skill demonstration

Run against the PR source at `76d944f97d267fd14ffb802942c7dbe64c259cbd` using the existing PR-head binary (`quoin 0.31.0-46-g42c9b80e-dirty`). All files were written only to an isolated scratch repo under `/var/tmp`; the PR worktree was not modified by the run. The fixture pack is synthetic and declares only the key `example.invalid/std@1` and clause IDs `EX-1`, `EX-2`.

`quoin module install` exited 0 from the scratch pack path. `quoin module list` reported the module as `pack: true`. The repo's `.ix/source-profile.yaml` added exactly `example.invalid/std@1`.

The profile evaluation returned:

```json
{"context":{},"profile":"security-demo","schemaVersion":"source-profile-evaluation-v1","sets":[{"key":{"authority":"nist","id":"sp800-53","version":"5.2.0"},"layer":"profile","status":"unavailable"},{"key":{"authority":"example.invalid","id":"std","version":"1"},"layer":"user","report":{"clauseSet":{"authority":"example.invalid","id":"std","version":"1"},"clauses":[{"clauseId":"EX-1","expectedOutputs":[],"force":"mandatory","outcome":"binding","reasons":[]},{"clauseId":"EX-2","expectedOutputs":[],"force":"mandatory","outcome":"binding","reasons":[]}],"context":{},"schemaVersion":"clause-binding-v1"},"status":"evaluated"}],"summary":{"clauses":{"binding":2,"not_binding":0,"unresolved":0},"setsEvaluated":1,"setsUnavailable":1}}
```

`quoin write --types control --json` returned the local control skeleton. Following the skill's typed-artifact process, the demo authored one scratch control whose complete Mappings table is:

| Set | Clause |
|---|---|
| example.invalid/std@1 | EX-1 |

No title, description, quotation or paraphrase from the pack was copied into the artifact. No standards text was consulted or reproduced.

`quire validate --scope <scratch-repo> "spec/security/*.md"` exited 0. It emitted only these semantic warnings: an unused imported module schema, defaulted local source identity, and unchecked OCL clause language. The positive mapped ID came from the installed exact set and is not inferred from a similar name. The NIST profile set remained unavailable and was not mapped.
