<!-- reviewer-dispositions repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113203417 round=1 model=gpt-6.1-sol run=24afab22-e2b5-4a8c-819c-7a12be467b04 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | All entries for each bound symbol are checked for fail/error. |
| FND-002 | fixed | Non-NotFound read_dir errors now propagate as store errors. |
| FND-003 | fixed | A post-publish error rolls back the newly written destination or restores its prior contents. |
| FND-004 | still-open | A pass-then-skip duplicate still exports PASSED; the mapper checks only the first matching entry for warning outcomes. |

+++ [reviewer data]

```yaml
dispositions:
  - fnd: FND-001
    outcome: fixed
    after_excerpt: |-
      .flat_map(|symbol| {
          run.entries
              .iter()
              .filter(|entry| entry.symbol.as_str() == symbol.as_str())
      })
      .filter(|entry| matches!(entry.outcome, Outcome::Fail | Outcome::Error))
  - fnd: FND-002
    outcome: fixed
    after_excerpt: |-
      let read = match std::fs::read_dir(&resolved) {
          Ok(read) => read,
          Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
          Err(error) => return Err(io("list", directory)(error)),
      };
  - fnd: FND-003
    outcome: fixed
    after_excerpt: |-
      if let Err(publish_error) = publish() {
          return Err(rollback_output(path, parent, backup)
              .err()
              .map_or(publish_error.clone(), |rollback_error| {
                  format!("{publish_error}; rollback failed: {rollback_error}")
              }));
      }
  - fnd: FND-004
    outcome: still-open
    reason: "In rust/crates/quoin-evidence/src/export/test_result.rs:162-170, .find(...) inspects only the first matching entry; a pass followed by skip exits 0 with result PASSED and the obligation in passedTests."
```

+++ 
