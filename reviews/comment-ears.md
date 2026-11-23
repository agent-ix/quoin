<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.36.8 module=spec-artifacts-process@v0.36.0 id=SR-17097 method=spec-review/ears-analysis lang=spec pr=quoin#1037 model=claude-opus-5-5 run=bd453c76-7802-4059-9c9d-b6621a2a6091 -->

Clean: 1/1 grammar-clean; compound and meta wording is recorded in SR-17096 FND-006 and FND-007.

+++ [reviewer data]

```yaml
clean: true
scope:
- id: FR-191 Release chain bullet 1
  path: spec/functional/FR-191-dossier-command.md
  role: examined
  excerpt: '`export` SHALL pass `--release <TAG>` to `quire export` unchanged and SHALL NOT rewrite, resolve or verify the tag; the release commit is the commit the tag names, and the dossier is committed after it.'
- id: FR-191 Release chain bullet 2
  path: spec/functional/FR-191-dossier-command.md
  role: examined
  excerpt: The dossier is committed after the tag because the `DOS-REL` document needs the EA `release-verify` result ([FR-201](./FR-201-dossier-release-record-content.md)), which needs the tag to exist. The resolved `HEAD` is therefore a descendant of the tagged release commit. It satisfies `quire export --release` when the commits between the tag and `HEAD` add or modify only files under the exported dossier's directory (quire-cli FR-034 section A); `export` SHALL leave that decision to `quire export` and report its refusal as exit 4, the report carrying `quire` exit code 3.
- id: FR-191 Release chain bullet 3
  path: spec/functional/FR-191-dossier-command.md
  role: examined
  excerpt: 'The repository SHALL hold a test that runs the whole chain once: an EA release-verify over a real tag, `quoin dossier generate` with its `--release-verify` result, a commit of the dossier files, and `quoin dossier export --release <TAG>` through a real `quire export`.'
findings: []
bindings: []
```

+++
