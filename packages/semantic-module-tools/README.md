# @agent-ix/semantic-module-tools

The shared emit driver for Quire semantic-module repositories. It runs the
official `@typespec/json-schema` emitter through `tsp compile`, keeps this
module's namespace, rewrites relative `$id`/`$ref` to absolute URLs under a
declared base (this module, semantic-core, or an imported module), and writes
`<package-dir>/schemas/`. It is a driver, not an emitter.

```
semantic-module-emit --package-dir my_module            # regenerate
semantic-module-emit --package-dir my_module --check    # fail on drift
```

| Flag                      | Default                                               |
| ------------------------- | ----------------------------------------------------- |
| `--package-dir`           | required                                              |
| `--repo-root`             | current directory (holds `node_modules`, `typespec/`) |
| `--source-dir`            | `<repo-root>/typespec`                                |
| `--manifest`              | `<package-dir>/manifest.yaml`                         |
| `--identity`              | `package.json` name without the leading `@`           |
| `--semantic-core-version` | installed `@agent-ix/semantic-core` version           |

The library entry exports `emit`, `filterModuleSchemas`, `absolutizeSchemas`,
`importedBases`, `checkOutput` and `writeOutput`. The repository using it
supplies `@typespec/compiler`, `@typespec/json-schema` and
`@agent-ix/semantic-core` as its own dependencies.
