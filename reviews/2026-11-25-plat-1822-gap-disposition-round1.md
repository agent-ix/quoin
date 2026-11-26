<!-- reviewer-dispositions repo=agent-ix/quoin visibility=public quoin=0.36.8 module=spec-artifacts-process@v0.36.0 id=SR-967175026302197768900474181941935084787 round=1 head=4dfd855ee07433cd9de9ee0821f038f56ebf4d5b mergeable=no model=gpt-6.2-sol run=886b79a1-05b8-47d6-822a-e56d12cf1734 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | The new fallback test populates IX_HOME and HOME/.ix with identical ten-module registries, then asserts only a count of ten for all three cases. An implementation ignoring IX_HOME and always selecting ~/.ix passes every assertion. Make the roots observably distinct, or populate HOME/.ix only after the two IX_HOME assertions. |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: The new fallback test populates IX_HOME and HOME/.ix with identical ten-module
    registries, then asserts only a count of ten for all three cases. An implementation
    ignoring IX_HOME and always selecting ~/.ix passes every assertion. Make the roots
    observably distinct, or populate HOME/.ix only after the two IX_HOME assertions.
```

+++
