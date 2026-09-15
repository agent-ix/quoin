# Native Quoin release contract

The native release artifact is named **`quoin`**. The CLI calls the governed
Rust runtime in-process; the temporary `quoin-core` protocol executable is
retained only for wire-parity evidence until the Stage 9 deletion cutover.
The artifact contract is independent of the Rust crate name `quoin-cli`.

Every stable GitHub Release carries `quoin-update-manifest.json` and one asset
per supported target:

| Target | Asset | Archive member |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `quoin-v{version}-x86_64-unknown-linux-gnu.tar.gz` | `quoin` |
| `x86_64-apple-darwin` | `quoin-v{version}-x86_64-apple-darwin.tar.gz` | `quoin` |
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
The existing npm package remains the staged installer until the separate #396
oclif/plugin disposition is made.
