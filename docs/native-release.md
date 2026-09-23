# Native Quoin release contract

The native release artifact is named **`quoin`**. The CLI calls the governed
Rust runtime in-process.
The artifact contract is independent of the Rust crate name `quoin-cli`.

Every stable GitHub Release carries `quoin-update-manifest.json` and one asset
per supported target:

| Target | Asset | Archive member |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `quoin-v{version}-x86_64-unknown-linux-gnu.tar.gz` | `quoin` |
| `aarch64-unknown-linux-gnu` | `quoin-v{version}-aarch64-unknown-linux-gnu.tar.gz` | `quoin` |
| `aarch64-apple-darwin` | `quoin-v{version}-aarch64-apple-darwin.tar.gz` | `quoin` |
| `x86_64-pc-windows-msvc` | `quoin-v{version}-x86_64-pc-windows-msvc.zip` | `quoin.exe` |

The manifest has `schema_version: 1`, a stable SemVer `version`, and one
record per target containing `target`, `asset`, `archive` (`tar-gz` or `zip`),
`size`, lowercase SHA-256 `sha256`, and HTTPS asset `url`. Archives contain
exactly one regular member. The default updater endpoint is the GitHub
`releases/latest/download/quoin-update-manifest.json` URL, so prereleases are
not candidates for ordinary updates.

`quoin update --registry <url>` retains its legacy role as an explicit custom
update source, but the URL now names a manifest rather than an npm registry.
## Initial installation

Download the archive for the host target from the stable GitHub Release, verify
it against `quoin-update-manifest.json`, extract its sole executable, and place
that executable on `PATH`. The native `quoin update` command then manages later
updates from the same manifest contract. The release workflow smokes the
extracted archive on every supported target before publishing it.

## Install with npm

```bash
npm install -g @agent-ix/quoin
```

`@agent-ix/quoin` is a thin launcher; installing it pulls in the matching
platform package as an `optionalDependency`:

| Platform | Package |
| --- | --- |
| linux-x64 | `@agent-ix/quoin-linux-x64` |
| linux-arm64 | `@agent-ix/quoin-linux-arm64` |
| darwin-arm64 | `@agent-ix/quoin-darwin-arm64` |
| win32-x64 | `@agent-ix/quoin-win32-x64` |

The Linux packages need glibc >= 2.35 (Ubuntu 22.04 or newer, and most
current distributions); they declare `libc: ["glibc"]` so npm refuses to
resolve them on a musl host instead of installing a binary that will not run.

Under an npm install, `quoin update` given as the first argument defers to
npm: it makes no change to the installation and reports the
`npm install -g @agent-ix/quoin@latest` command to run instead, since npm
alone owns the files it placed under `node_modules`. The launcher only
recognises `update` as the first argument — an invocation such as
`quoin --no-project-config update` is not intercepted and reaches the
native updater; closing that gap needs npm-install detection inside the
native updater itself, tracked as PLAT-1013.

## Packaging the npm distribution

`.github/workflows/release.yml` (`workflow_dispatch`, inputs `tag` and
`publish`) repackages an existing GitHub Release's already-smoked assets into
the npm packages above; it never rebuilds the binary, and the npm version
always equals the release tag. `publish: false` (the default) verifies and
packages without publishing; `publish: true` publishes to public npm and
smokes the published packages. See
[FR-112](../spec/functional/FR-112-npm-distribution-from-release-assets.md).
