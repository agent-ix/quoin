# Refreshing the verification stack

The maintainer integrating a producer or declaration change owns the refresh.
Do not loosen source checks when a sibling checkout advances. Historical v1 names
seven independent sources: Quoin, Quire engine, Quire CLI, qa-corpus,
filament-ide-rs (read-only Tier-2 corpus), spec-artifacts-process and
spec-artifacts-iso. The CLI's Cargo pin must select that exact engine.

## Explicit declaration policy (v2)

V2 adds one source, `engineering-assurance`, and requires exactly these eight
repository keys. It is an explicit reviewed input to `--lock`, never an implicit
upgrade of the accepted v1 lock. Both modes preserve historical producer cohorts;
v1 replay does not claim the declaration isolation supplied by v2.

V2 has exactly one declaration set, `declarations.quoinValidation`, ordered as:

1. `spec-artifacts-process` / `spec_artifacts_process`
2. `spec-artifacts-iso` / `spec_artifacts_iso`
3. `engineering-assurance` / `engineering_assurance`

Each entry contains `repository`, `path`, `tree` (the full Git tree object ID),
and `files`: a complete path-sorted array of `{path, mode, digest}` records.
Paths are relative to the module subdirectory; modes are `100644` or `100755`;
digests are full `sha256:` values. `manifest.yaml` must be present. There are no
absolute workstation paths in this policy. Relocking recomputes the tree and
every file digest from the selected source commit, including nested schemas and
templates. The reviewed input must already be a structurally complete v2 policy;
missing inventory is not an invitation to discover a default module set.

Add `--root engineering-assurance=/absolute/engineering-assurance` to the relock
command below and select the reviewed v2 policy with `--lock /absolute/policy.json`.
Use the released EA v0.2.0 commit `8ea16ce240934aa2c31c1cc3f781b7eb0f8c73ba`
for this campaign; keep the historical Filament source
`546e7943ee5a8fe552242cbb19d12aa902536652`. A moving upstream main is not a
replacement policy. EA's separate accepted cross-component compatibility matrix
does not automatically accept these newer engine/CLI/Quoin source candidates.

Canonical v2 preflight verifies all inventories and materializes literal Git
objects into fresh scratch directories. Non-regular entries, missing files,
changed modes/digests and trees are refused; ignored working files, checkout
filters and Git archive export attributes cannot alter those bytes. The existing
attestation's lock digest binds this complete declaration inventory without
changing the attestation schema.

The canonical runner passes a scratch JSON array of exact roots through
`QUOIN_VERIFICATION_DECLARATIONS` to the existing Make `validate` prerequisite.
That route calls the native Quire validator with repeated `--module` flags,
isolated `IX_HOME` and no module discovery environment. It does not call
`ensure-defaults`. An explicit empty or malformed root manifest fails. Ordinary
validation without this explicit manifest retains its existing default installer
behavior. The manifest is internal invocation data, not accepted evidence.

This slice changes Quoin's own static validation only. Tier-1 case declarations,
Tier-2 historical declaration sets and external producer feature support are
unchanged; their explicit repeated-module join is a separate reviewed slice.

## Prepare a candidate

Install the locked developer dependencies with
`corepack pnpm install --frozen-lockfile` before invoking the relock command.
The canonical verifier retains its own frozen install after source checks, so
stale-source diagnostics still work on a checkout without `node_modules`.

First commit the source changes, initialize Quoin's `corpus` submodule to the
selected qa-corpus commit, and arrange clean checkouts of all seven sources.
Each selected commit must already be reachable from a remote-tracking ref.
Fetch and review source changes separately: this command never fetches, merges,
installs or publishes anything.

```sh
make verification-relock RELOCK_ARGS='--out /tmp/stack-candidate.json --root quoin=/absolute/quoin --root quire=/absolute/quire-rs --root quire-cli=/absolute/quire-cli --root qa-corpus=/absolute/quoin/corpus --root filament-ide-rs=/absolute/filament-ide-rs --root spec-artifacts-process=/absolute/spec-artifacts-process --root spec-artifacts-iso=/absolute/spec-artifacts-iso'
```

All roots are required; no sibling, PATH or installed-module discovery occurs.
Cargo TOML is parsed with the exact locked `smol-toml` development dependency;
only one explicit normal `quire-rs` dependency (including a package alias) is
accepted. Branch, tag, path, workspace inheritance and engine override ambiguity
are refused. Contract provenance is read from the exported `QUIRE_CONTRACT`
literal using the already-pinned TypeScript parser, never from comments or by
executing the module. Each candidate artifact must equal its committed Git blob;
`corpus/` files resolve through the selected corpus gitlink. Hidden working-tree
changes cannot be blessed merely because `git status` is empty.
Every tracked regular file, executable mode and symlink is additionally checked
against its Git blob for each selected source. Gitlinks remain independent
source identities validated at their consuming boundary. Inventory computation
materializes the complete selected QA tree from literal Git blobs into an
isolated directory and runs its native reader with isolated Python import
settings. Ignored working-tree inputs never enter that snapshot. The snapshot
refuses symlinks and nested gitlinks rather than resolving unpinned inputs; the
current corpus has neither. Snapshot materialization has an explicit 128 MiB
batch-output bound and fails rather than truncating a larger corpus.
The output must not exist. The command derives revisions, artifact hashes and
case counts, not policy. If the QA population exceeds the locked Tier-1 timeout
budget, explicitly review that budget in the input lock before trying again.
An optional `--lock` selects the reviewed input policy instead of the committed
lock. The output is a candidate, not a successful verification attestation.

Vendored output schemas must equal the bytes at both
`QUIRE_CONTRACT.sourceRevision` and the selected engine revision. A newer engine
with unchanged schemas may retain the older, exact contract source. Otherwise
update `src/quire/contract.ts` deliberately and run
`node scripts/refresh-quire-schemas.mjs --source /absolute/quire-rs`, then review
and commit the schema and consumer-test changes before relocking.

## Replay and promotion

Review the candidate diff. Preserve the two historical Quoin cohorts: the
engine benchmark population and the QA external producer are distinct from the
current Quoin under test. In particular, do not rewrite QA expectations to fit
an ambient Quoin binary. The canonical runner builds the locked historical
external producer itself.

After required source reviews and merges, prepare exact clean source checkouts,
commit the candidate as the lock-only evidence overlay on its recorded Quoin
code commit, and run the existing canonical `make test` with the seven source
routes (`QUIRE_ROOT`, `QUIRE_CLI_ROOT`, `QA_CORPUS_ROOT`,
`FILAMENT_IDE_RS_ROOT`, `SPEC_ARTIFACTS_PROCESS_ROOT`,
`SPEC_ARTIFACTS_ISO_ROOT`; Quoin is the runner checkout).
V2 additionally requires `ENGINEERING_ASSURANCE_ROOT` at its exact released
commit. The same source, artifact, schema, capability and native gate checks apply
to both modes; preparing v2 metadata does not remove any review or promotion gate.
Use `make bench-tier1-update` only for a separately reviewed evidence refresh;
it is not the relock command. Retain native producer failures and incomparable
populations, regenerate evidence through the existing measurement path, and
review the resulting overlay before promotion. No command here authorizes a
publication or bypasses human review.

The process module's functional matrix header and coverage selector must agree
at their owning declarations. Exact module selection and strict structural
diagnostics remain required; relocking cannot repair a contradictory module.
