<!-- reviewer-dispositions repo=agent-ix/quoin visibility=public quoin=0.36.8 module=spec-artifacts-process@v0.36.0 id=SR-967175026302197768900474181941935084787 round=2 head=251504911431eb495f24c2dd02f52b5aae1e0db4 mergeable=yes model=gpt-6.2-sol run=a7f42098-64bf-482d-90c2-8096151c77b4 -->

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Default home is populated only after both IX_HOME assertions; selecting the wrong root now fails. |

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'let unset_config_root = invoke(None, Some(&legacy_home), &process_home,
    &["module", "list"]);

    assert_module_count(&unset_config_root, 10, "IX_HOME after unset IX_CONFIG_ROOT");


    install_defaults(&default_home);

    let default = invoke(None, None, &process_home, &["module", "list"]);

    assert_module_count(&default, 10, "~/.ix after both overrides are unset");'
```

+++
