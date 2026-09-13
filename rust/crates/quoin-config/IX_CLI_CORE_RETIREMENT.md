# Retiring `@agent-ix/ix-cli-core` — the symbol inventory

Deliverable of quoin#381 (EPIC #373, Stage 7). The EPIC records "quoin uses 9 of
its symbols"; **the verified count for `src/` is 11** (10 values + 1 type), plus
one further symbol used only by `tests/`. The inventory below was taken from the
import sites and checked against ix-cli-core 0.12.0's `dist/index.js`, not from
its `.d.ts` alone — three behaviours differ between the two.

`@agent-ix/ix-cli-core` is ~2,530 lines. quoin uses none of its auth, secrets,
marketplace or capability-resolution code. Nothing is ported wholesale.

## Used by `src/` — 11 symbols

| # | Symbol | Import site | What it does | Rust successor |
|---|---|---|---|---|
| 1 | `ConfigService` | `src/org.ts:4` | `forPlugin(id, schema, opts)` builds a layered reader over `<configRoot>/config.d/<id>.yaml` (user) and `<projectRoot>/config.d/<id>.yaml` (project); `.get()` deep-merges defaults < user < project < env bindings and validates once, **never throwing** — a broken file yields schema defaults plus an in-memory incident for `config doctor`. `.set()` merges, validates, then writes atomically under an advisory lock. | `quoin_config::service::ConfigService` — same layering and same totality; incidents are an owned `IncidentLog` rather than a module global. |
| 2 | `registerPluginSchema` | `src/config-schema.ts:1` | Fills the in-process table a host's init hook populates from each loaded plugin's `ixSchema` export. Refuses a **non-strict** schema; a duplicate is a non-throwing failure that preserves the first entry. | `quoin_config::plugin_registry::PluginSchemaRegistry` + `register_quoin_schema`. |
| 3 | `runConfigGet` | `src/commands/config/get.ts:2` | Resolves the plugin, descends a dotted key through `get()`, prints the value or reports it unset — **exit 0 either way**. | `ConfigService::get_key` (the read); rendering belongs to Stage 9. |
| 4 | `runConfigSet` | `src/commands/config/set.ts:2` | Classifies the key's type off the zod shape (scalar → raw string, array/object → `JSON.parse`), sets it, validates, writes. The strict schema is what turns an unknown key into a refusal. | `ConfigService::set_key`, with the `KeyKind` classification moved onto the `PluginConfigSchema` trait. **Deliberate divergence:** the TS handler writes back the whole *resolved* object; the Rust one writes only the named key. |
| 5 | `runConfigDoctor` | `src/commands/config/doctor.ts:1` | Walks every registered plugin's **user-level** file (project files are invisible to it), reporting valid / invalid / unregistered, plus recent incidents. Exit 1 only when something is invalid. | `ConfigService::doctor` → `DoctorEntry` / `DoctorStatus`; the report assembly and exit mapping belong to Stage 9. |
| 6 | `runConfigEdit` | `src/commands/config/edit.ts:1` | Materializes the file via `set({})`, spawns `$VISUAL`/`$EDITOR`/`vi` on it, then re-validates and rethrows on a bad edit. | **Not ported here.** Spawning an interactive editor is a CLI-shell concern (Stage 9). `ConfigService::set`/`get` supply the materialize-and-revalidate halves it needs. |
| 7 | `BaseCommand` | `src/base.ts:1` | oclif `Command` subclass owning `--config-root` / `--no-project-config`, publishing `{configRoot, projectConfigRoot: <cwd>/.ix, projectConfigEnabled}` before each run. | The **context** it publishes is `quoin_config::paths::RuntimeContext` (`RuntimeContext::for_cwd`). The oclif base class itself is Stage 9. |
| 8 | `loadConfig` | `src/cli.ts:5` | One line: `Config.load()` from `@oclif/core`. Resolves the command graph without dispatching. | Stage 9 (clap). No Rust successor here. |
| 9 | `run` | `src/cli.ts:5` | One line: `oclifRun(argv, options)`. Dispatches; does not exit. | Stage 9. |
| 10 | `RunnerLoadOptions` (type) | `src/cli.ts:5` | Alias for oclif's `Interfaces.LoadOptions`. | Stage 9. Type-only; nothing to reimplement. |
| 11 | `maybeOfferUpdate` | `src/base.ts:1` | Throttled `npm view` update check with a TTL cache; skips in CI, when `NO_UPDATE_NOTIFIER` is set, or when not a TTY; never throws into the host. | **Not quoin's to own.** Update notification is a distribution concern; Stage 9 decides whether the clap shell keeps it. |
| 12 | `runSelfUpdate` | `src/commands/update.ts:2` | `npm view` then `npm install -g <pkg>@<latest>`, with scope-aware registry flags. | As above — Stage 9. |

> The table lists 12 rows because `RunnerLoadOptions` is a type import sharing
> `src/cli.ts:5` with `loadConfig` and `run`. Distinct **symbols** imported by
> `src/`: 11 values-or-types across 6 files.

`UnknownPluginError` appears only in a doc comment (`src/config-schema.ts:52`),
never imported; its behaviour is reproduced as
`quoin_config::ConfigError::UnknownPlugin` because the registry must be able to
raise it.

## Used only by `tests/` — 1 further symbol

| Symbol | Site | What it does |
|---|---|---|
| `listCorePlugins` | `tests/it-005-sync-discovery.test.ts:8` | Lists non-root `core`-type oclif plugins with their command ids. Used to assert the `@agent-ix/filament-plan-sync` extension contract still resolves. Stage 9 territory; see the plugin-contract note below. |

## Three places the implementation contradicts the published types

A reimplementation driven by the `.d.ts` alone would be wrong in these three
ways. All three were checked against `dist/index.js` at 0.12.0.

1. `registerPluginSchema`'s return type advertises `kind: "idempotent"`. The
   0.12.0 body **never returns it** — a second registration of the same package
   is `{ ok: false, kind: "duplicate-registration" }`. `RegistrationOutcome`
   follows the body.
2. `ConfigService`'s config format is **YAML at `~/.config/ix/config.d/<id>.yaml`**,
   not JSON and not under `~/.ix/`. `~/.ix` is the *module* home
   (`quoin-modules`), a different tree entirely.
3. `readRegistry` (ts-plugin-kit, same family of surprise) documents tolerance
   of a malformed file but lets `JSON.parse` throw.

## What retiring the dependency still needs

`quoin-config` and `quoin-modules` cover rows 1–5 and the context of row 7.
Rows 6 and 8–12 are the **oclif shell**, which Stage 9 owns. The dependency can
be dropped from `package.json` only once Stage 9 lands, not before.
