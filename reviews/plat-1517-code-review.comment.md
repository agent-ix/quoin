<!-- reviewer repo=agent-ix/quoin visibility=public quoin=0.30.1 module=spec-artifacts-process@v0.31.0 id=SR-20261113014535 method=code-review lang=rust pr=quoin#840 model=claude-opus-5-5 run=bee49e35-0666-45b9-ba47-c1838e0c67e0 -->

Code review + rust-review of quoin#840 at 084c12bb. Verdict: CONDITIONAL. `tc_1517` 10/10 pass, clippy -D warnings and fmt clean on quoin-cli. Mergeable once FND-001 is fixed and FND-002..005 are fixed or disposed.

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | medium | other | `as_of_date` adds a second RFC 3339 reader (`time` + new `parsing` feature) beside the workspace's one reader `quoin_store::instant` (PLAT-91); use `Instant::parse` + `from_unix_timestamp` and drop `parsing` |
| FND-002 | low | soundness | Unknown entry keys silently ignored; repo untrusted-input idiom says refuse |
| FND-003 | low | code-bug | Decoding the whole base makes `pkg:x/a%2Fb` and `pkg:x/a/b` the same identity (match + false duplicate) |
| FND-004 | low | other | `SupportError` names the entry in prose only; no context map, no code round-trip/uniqueness test; tests match `contains("entry 2")` |
| FND-005 | low | other | `#[allow(dead_code)]` on `mod support` should be `#[expect(dead_code)]` so PLAT-1518 is forced to remove it |

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: rust/crates/quoin-cli/src/soup/support.rs, path: rust/crates/quoin-cli/src/soup/support.rs, role: examined, excerpt: "Supplied support data for `quoin soup report` (FR-143): the support file, its validation, and the pure evaluation of one record against it."}
  - {id: rust/crates/quoin-cli/Cargo.toml, path: rust/crates/quoin-cli/Cargo.toml, role: examined, excerpt: "time = { workspace = true, features = [\"parsing\"] }"}
  - {id: rust/crates/quoin-cli/src/soup.rs, path: rust/crates/quoin-cli/src/soup.rs, role: examined, excerpt: "#[allow(dead_code, reason = \"`quoin soup report` (PLAT-1518) is the caller\")]\nmod support;"}
  - {id: rust/crates/quoin-cli/src/soup/plan.rs, path: rust/crates/quoin-cli/src/soup/plan.rs, role: examined, excerpt: "pub(super) fn percent_decode(text: &str) -> String {"}
  - {id: rust/crates/quoin-store/src/instant.rs, path: rust/crates/quoin-store/src/instant.rs, role: context_only, excerpt: "The one timestamp reader, with a named grammar per language (PLAT-91)."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: other
    artifact_id: rust/crates/quoin-cli/src/soup/support.rs
    related_ids: [rust/crates/quoin-cli/Cargo.toml, rust/crates/quoin-store/src/instant.rs]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "120-132"
    excerpt: |-
      OffsetDateTime::parse(text, &Rfc3339)
          .map(|instant| instant.to_offset(UtcOffset::UTC).date())
    finding: "as_of_date parses instants with time's Rfc3339 parser and Cargo.toml enables time's `parsing` feature for it. The workspace has one timestamp reader, quoin_store::instant (PLAT-91), and quoin-cli already depends on quoin-store. Use Instant::parse(text, InstantGrammar::Rfc3339 or CalendarInstant), epoch_millis(), then OffsetDateTime::from_unix_timestamp(ms.div_euclid(1000))?.date(); that needs only default time features, so drop features = [\"parsing\"]. Keep the strict YYYY-MM-DD path for a bare date."
  - fnd: FND-002
    severity: low
    confidence: medium
    method: code-review
    check: soundness
    artifact_id: rust/crates/quoin-cli/src/soup/support.rs
    related_ids: [FR-143]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "264-314"
    excerpt: |-
      let field = |name: &str| item.get(name).and_then(Value::as_str);
    finding: "parse_entry silently ignores keys other than purl, support_status, end_of_support and source (author gap 3). quoin rust-style denies unknown fields on untrusted input. Refuse such an entry under a new code (e.g. soup.support_unknown_field) naming the entry and key, and add a case to tc_1517_001."
  - fnd: FND-003
    severity: low
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: rust/crates/quoin-cli/src/soup/support.rs
    related_ids: [rust/crates/quoin-cli/src/soup/plan.rs]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "219-223"
    excerpt: |-
      fn identity(purl: &Purl) -> (String, Option<String>) {
          (percent_decode(&purl.base), purl.version.clone())
      }
    finding: "identity percent-decodes the whole pkg:type/ns/name string, so an encoded slash in a name collides with the namespace/name split: pkg:x/a%2Fb (name a/b) and pkg:x/a/b (namespace a, name b) get one identity, match each other and are refused as duplicates. Compare type, namespace and name segments separately, each decoded."
  - fnd: FND-004
    severity: low
    confidence: medium
    method: code-review
    check: other
    artifact_id: rust/crates/quoin-cli/src/soup/support.rs
    related_ids: [rust/crates/quoin-cli/src/soup.rs]
    path: rust/crates/quoin-cli/src/soup/support.rs
    lines: "134-198, 559-570"
    excerpt: |-
      pub(super) struct SupportError {
          pub(super) code: SupportErrorCode,
          pub(super) message: Box<str>,
      }
    finding: "SupportError names the refused entry only in message prose. It has no context map like soup.rs's soup.* diagnostics, and no all() with code round-trip/uniqueness tests as the quoin error-envelope pattern requires. PLAT-1518 cannot carry entry position and purl as diagnostic context without parsing prose, and tests assert message.contains(\"entry 2\"), which \"entry 20\" also satisfies. Add context (entry, purl, other_entry for duplicates), assert on it, and add the code test."
  - fnd: FND-005
    severity: low
    confidence: medium
    method: code-review
    check: other
    artifact_id: rust/crates/quoin-cli/src/soup.rs
    related_ids: []
    path: rust/crates/quoin-cli/src/soup.rs
    lines: "12"
    excerpt: |-
      #[allow(dead_code, reason = "`quoin soup report` (PLAT-1518) is the caller")]
    finding: "The allow is justified (PLAT-1518 is the caller and is blocked by this ticket) but would linger silently once the module is used. #[expect(dead_code, reason = ...)] fails the build then and forces removal; it is the only dead_code suppression in the workspace."
bindings: []
```

+++
