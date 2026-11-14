<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203422 method=spec-review/dependency lang=spec pr=quoin#888 model=gpt-6.1-sol run=2a67d7d7-f270-4a7d-8a5d-5db8116c4e48 -->

+++ [reviewer data]

```yaml
clean: true
scope:
  - {id: FR-185, path: spec/functional/FR-185-in-toto-test-result-export.md, role: examined, excerpt: "FR-185 references the Quoin command boundary FR-068 and the Engineering Assurance consumer contract FR-069."}
  - {id: FR-068, path: spec/functional/FR-068-producer-facing-change-assurance-cli-surface.md, role: context_only, excerpt: "The existing producer-facing CLI boundary that this export extends."}
  - {id: FR-069, path: spec/functional/FR-069-test-result-export-is-consumed-as-data.md, role: context_only, excerpt: "The downstream consumer contract adopted by FR-185."}
findings: []
```

+++
