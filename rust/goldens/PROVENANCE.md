# Golden provenance — quoin#381 (Stage 7: config, plugins, modules)

The TypeScript implementation is the oracle **exactly once**: these files were
captured from it on 2026-09-12 and committed. No Rust test in `rust/` shells out
to Node, to `tsx`, or to `vitest`. A test that disagrees with a golden is a
parity finding to adjudicate, not a file to regenerate casually.

## Source revision

| Fact | Value |
|---|---|
| Repository | `agent-ix/quoin` |
| Revision the goldens were produced at | `7c8e18f873b6295f528b4d694647a10d82cfb59c` (branch `spec/373-rust-burn-down`) |
| Branch this port is based on | `main` @ `4d27dcf1621d8c28da0961a5521a5be7b6d1cd28` |

The oracle source files —
`src/config-schema.ts`, `src/org.ts`, `src/plugins.ts`, `src/modules.ts`,
`src/catalog.ts` — are **byte-identical** between those two revisions
(`git diff 4d27dcf 7c8e18f -- <those paths>` is empty), so capturing at `7c8e18f`
is capturing `main`'s behaviour.

`default-modules.yaml` is the one input that differs between them: at `7c8e18f`
the `spec-artifacts-process` entry is pinned to `d605caa…` where `main` has
`375fc2a…`. `default-modules.yaml` in this directory is the exact file the
goldens were captured from, and `tc_381_233` parses **that** committed copy, so
the manifest golden and its input cannot drift apart. Nothing here asserts a
particular upstream pin — only that the parse produces the same entries the
TypeScript validator produced.

## Dependency versions at capture time

| Package | Version |
|---|---|
| `@agent-ix/ix-cli-core` | 0.12.0 |
| `@agent-ix/ts-plugin-kit` | 0.2.0 |
| `zod` | 4.4.3 |
| `yaml` | 2.9.0 |
| Node | v22.15.0 |

## How they were captured

`capture-ts-oracle.mjs` in this directory is the capture script, committed
verbatim. It imports the real `src/` modules and the real `@agent-ix/ts-plugin-kit`
exports and records their answers. It was run as a Vitest test file, because the
`src/` modules use `.js`-suffixed ESM specifiers that plain `node
--experimental-strip-types` will not resolve:

```bash
# from a checkout of agent-ix/quoin at the revision above, with pnpm install run
cp rust/goldens/capture-ts-oracle.mjs tests/.capture-381.test.ts
#   …then adjust the four `../src/*.ts` specifiers to drop the `.ts` suffix and
#   replace the trailing `writeFileSync(process.argv[2], …)` line with
#   `test("capture", () => { writeFileSync(process.env.QUOIN_381_OUT, …); });`
QUOIN_381_OUT=rust/goldens/ts-oracle.json npx vitest run tests/.capture-381.test.ts
rm tests/.capture-381.test.ts
```

## What is in `ts-oracle.json`

| Section | Oracle | Consumed by |
|---|---|---|
| `origin_org_urls`, `origin_org_configs` | `src/org.ts` `originOrg` | `quoin-config` `tc_381_100`, `tc_381_101` |
| `config_service_org`, `resolve_org_flag_env` | `src/org.ts` `resolveOrg` (through `ix-cli-core`'s `ConfigService`) | `quoin-config` `tc_381_103`, `tc_381_104` |
| `config_schema` | `src/config-schema.ts` `QuoinConfigSchema` | `quoin-config` `tc_381_102` |
| `parse_source_arg` | `src/plugins.ts` `parseSourceArg` | `quoin-modules` `tc_381_230` |
| `to_git_url`, `normalize_source` | `ts-plugin-kit` `toGitUrl`, `normalizeSource` | `quoin-modules` `tc_381_231`, `tc_381_232` |
| `default_modules_manifest`, `validate_manifest` | `ts-plugin-kit` `validateMarketplaceManifest` | `quoin-modules` `tc_381_233`, `tc_381_234` |
| `registry_file_bytes`, `registry_file_bytes_full`, `registry_read_*` | `ts-plugin-kit` `writeRegistry` / `readRegistry` | `quoin-modules` `tc_381_235`, `tc_381_236`, `tc_381_239` |
| `read_module_name` | `src/plugins.ts` `readModuleName` | `quoin-modules` `tc_381_237` |
| `paths` | `src/catalog.ts`, `src/plugins.ts` | `quoin-modules` `tc_381_238` |

## Deliberate divergences from the oracle

Each of these is a place the Rust port does **not** match the captured
behaviour, on purpose. They are listed here rather than only in code comments
because a later reviewer will reach for this file first.

1. **`{ org: null }` is refused.** zod's `.optional()` accepts `undefined` but
   not an explicit `null`; serde's `Option<T>` accepts both. `QuoinConfig`
   carries a `deserialize_with` that refuses a present-but-null `org`, so the
   Rust and TypeScript verdicts agree. Caught by `tc_381_102` before it was
   fixed, which is the golden set doing its job.
2. **Absent optionals are omitted, not written as `null`.** `parseSourceArg`
   returns `{ ref: undefined }`; the Rust `Source` omits the key. The registry
   *file* is unaffected — `JSON.stringify` drops `undefined` too, which
   `registry_file_bytes` pins byte for byte.
3. **Malformed registry JSON is an error, not an empty registry.**
   `readRegistry`'s doc comment promises tolerance but its body lets
   `JSON.parse` throw; the Rust port matches the body and states the reason:
   silently reporting "nothing installed" would destroy the snapshot
   `installPlugin`'s rollback depends on.
4. **`config set <key>` writes only that key.** `runConfigSet` starts from
   `get()`, so it persists the whole *resolved* object and can bake an
   environment- or project-supplied value into the user file as a side effect of
   setting an unrelated key. `ConfigService::set_key` writes the named key only.
5. **The source cache lives at `~/.ix/cache/quoin-modules`, not
   `~/.ix/cache/ts-plugin-kit`.** `ts-plugin-kit` keeps non-bare clones it drives
   with `git checkout` and `git sparse-checkout`; this crate keeps bare
   repositories with no worktree. The two layouts are incompatible, and sharing
   one directory during staged coexistence would corrupt whichever ran second.
6. **Symlink and submodule tree entries are skipped on extraction.** `write_tree`
   materializes blobs and trees only; an entry whose mode is a symlink or a
   gitlink is passed over without error. `git checkout` would recreate the
   symlink and record the submodule gitlink, so this is a real behavioural
   difference, not an implementation detail. It is deliberate for now: a module
   is a directory of specs, a symlink in an extracted module is an escape route
   out of the target directory that the path checks above would otherwise have
   to re-validate, and a submodule has no content in the fetched pack to
   materialize at all. A module that genuinely needs either currently installs
   with those entries missing rather than failing — which is the weaker half of
   this divergence, and the reason it is written down here.
7. **Unknown config keys are all reported, not just the first.**
   `#[serde(deny_unknown_fields)]` aborts deserialization at the first
   unrecognised key, where zod's `.strict()` collects every one. `QuoinConfig::validate`
   therefore scans the top-level mapping for unknown keys *before* handing the
   document to serde, and returns one `ConfigIssue` per key. This is parity with
   the oracle's *reporting*, reached by a different route than the oracle's; the
   verdict (invalid) is identical either way, and only the issue count differs
   from what `deny_unknown_fields` alone would produce. Nested objects still stop
   at serde's first unknown key — quoin's schema has no nested objects today, so
   nothing exercises that, and it is recorded here rather than claimed as closed.

## FR-027-AC-8 and delegation

`spec/functional/FR-027-*.md` AC-8 requires quoin's config handlers to
**delegate** to `@agent-ix/ix-cli-core`. `quoin-config` does not delegate: it
reimplements the nine `ix-cli-core` symbols quoin actually uses — plugin-id
derivation, the schema registry, the layered `ConfigService`, the incident log,
the advisory write lock, and org resolution — and depends on no TypeScript at
runtime.

That is the intended design, not an oversight. The Rust burn-down deliberately
does not port `@agent-ix/ix-cli-core`: quoin uses a small, well-bounded fraction
of it, and porting the whole package would mean owning a second CLI framework in
Rust to satisfy a dependency edge rather than a requirement. Reimplementing the
used surface against captured goldens is cheaper and is what the golden set in
this directory exists to keep honest.

The conflict with AC-8 is therefore real and is resolved as follows:

* **AC-8 governs the retained TypeScript implementation** for as long as it is
  the shipping one. Nothing in `src/` is exempted by this crate's existence, and
  the TypeScript handlers continue to delegate.
* **`quoin-config` is the divergence recorded here**, which is what makes it a
  decision rather than a defect.
* **Cutover must amend AC-8.** When the Rust implementation replaces the
  TypeScript one, AC-8 as written becomes false about the shipping code, and a
  spec that is false about the shipping code is worse than no spec. Amending
  AC-8 — to require the reimplemented surface to match ix-cli-core's *observable
  behaviour*, which is what the goldens already test — is a **named deliverable
  of the cutover ticket**, not a follow-up.

Amending the spec is out of scope for quoin#381 and is not done here: this file
records the conflict and its resolution so the cutover ticket inherits a written
decision instead of rediscovering an argument.
