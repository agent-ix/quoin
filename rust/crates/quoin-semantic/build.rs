// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Embed Quoin's semantic contract into native releases (quoin#527).
//!
//! Three families of schema make up the contract (FR-070, FR-073): one quoin
//! owns (`src/semantic/sweep-report.schema.json`), and two it depends on
//! rather than vendors -- `agent-ix/filament-core-data`'s module-manifest,
//! package-manifest and common schemas, and its semantic-core JSON Schema
//! bundle -- read straight out of the published npm packages
//! `@agent-ix/semantic-core` and `@agent-ix/filament-core-data`, resolved
//! through this repository's own `node_modules` (PLAT-887's de-vendoring;
//! `make workflow-assets` / `pnpm install` populates it, the same mechanism
//! `quoin-cli::flow` uses for `@agent-ix/ix-spec-workflows`). Nothing is
//! copied into this repository's own tree: `include_bytes!` reads the
//! installed package's files directly, at their own paths, and the generated
//! constant below only re-labels each one with the internal name
//! [`crate::contract`]'s path helpers already expect
//! (`schemas/semantic-core/<Type>.json`, and so on) -- a label the compiled
//! binary and [`crate::embedded::materialize_embedded_contract`] use to lay
//! the bytes out in a runtime cache dir, unrelated to the packages' own
//! directory layout.
//!
//! # module-manifest, package-manifest and common: blocked on publish
//!
//! `module-manifest.schema.json` is moving from `agent-ix/filament-core-service`
//! (a private, service repo -- not a dependency this public repo may take) to
//! `agent-ix/filament-core-data`, beside `package-manifest.schema.json` and
//! `common.schema.json` under `schema/semantic/v1/`. As of this writing
//! `@agent-ix/filament-core-data`'s only published version (0.1.0) does not
//! carry that directory yet -- it ships the unrelated Avro core-data contract.
//! Building a submodule, a copy, or a version pin to bridge that gap is
//! exactly the pattern this de-vendoring removes, so these three are OPTIONAL
//! embeds: if `node_modules/@agent-ix/filament-core-data/schema/semantic/v1/`
//! does not carry a file, this build script skips it (with a `cargo::warning`)
//! rather than failing, and the resulting binary is missing that one schema.
//! Every production code path that reads it already returns
//! [`crate::error::SemanticError::VendoredSchemaUnreadable`] on a missing
//! file, which is exactly the right, honest refusal here: "not yet published,"
//! not "corrupted." The root `package.json` still declares
//! `@agent-ix/filament-core-data` as a real dependency, pinned to its only
//! currently published version (`0.1.0`, the unrelated Avro core-data
//! contract) -- bump that pin once a version ships `schema/semantic/v1/` and
//! these three embeds become required like the rest. `semantic-core` has no
//! such gap --
//! `@agent-ix/semantic-core@0.1.0` already carries the exact bytes quoin
//! ships -- so it is a hard requirement, same as quoin's own
//! `sweep-report.schema.json`. The root `package.json` pins it to the exact
//! version `0.1.0` rather than a range: `0.2.0` already carries a breaking
//! schema change (a widened `ClauseLanguage` pattern, a new
//! `x-agent-ix-semantic-id` field) that [`crate::contract::SEMANTIC_CONTRACT`]
//! has not onboarded, and its digest assertions would rightly fail against
//! it. That digest, not the version string, is the real gate -- the pin just
//! avoids a predictable, unhelpful red the moment a compatible-looking
//! `^0.1.0` resolves to a version whose content is not.
//!
//! Explicit files, not a recursive walk: a walk over `node_modules/@agent-ix`
//! would embed unrelated packages into the quoin binary. The semantic-core
//! bundle's member files are the one exception, discovered by directory
//! listing rather than named one by one, because that list is exactly what
//! [`crate::contract::semantic_core_bundle_digest`] already computes the same
//! way -- listing it a second time here, by hand, would be the two sources of
//! truth this workspace's own idiom doc warns against.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// One embedded file: the internal name the runtime cache lays it out under,
/// and the absolute path `include_bytes!` reads it from.
struct Embed {
    internal_name: String,
    absolute_path: PathBuf,
    /// Required embeds fail the build when absent; optional ones (the three
    /// still blocked on `filament-core-data` publishing `schema/semantic/v1/`)
    /// are skipped with a `cargo::warning` instead.
    required: bool,
}

fn semantic_core_members(
    semantic_core_pkg: &Path,
    embeds: &mut Vec<Embed>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_schema_dir = semantic_core_pkg.join("generated/json-schema");
    let mut names = Vec::new();
    for entry in fs::read_dir(&json_schema_dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if Path::new(&name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        {
            names.push(name);
        }
    }
    names.sort();
    for name in names {
        embeds.push(Embed {
            internal_name: format!("schemas/semantic-core/{name}"),
            absolute_path: json_schema_dir.join(&name),
            required: true,
        });
    }
    embeds.push(Embed {
        internal_name: "schemas/semantic-core/toolchain.json".to_owned(),
        absolute_path: semantic_core_pkg.join("generated/toolchain.json"),
        required: true,
    });
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| std::io::Error::other("cargo did not set CARGO_MANIFEST_DIR"))?,
    );
    let repo_root = manifest.join("../../..");
    let node_modules_agent_ix = repo_root.join("node_modules/@agent-ix");
    let semantic_core_pkg = node_modules_agent_ix.join("semantic-core");
    let filament_core_data_pkg = node_modules_agent_ix.join("filament-core-data");
    let filament_core_data_semantic_v1 = filament_core_data_pkg.join("schema/semantic/v1");

    let mut embeds = vec![
        Embed {
            internal_name: "sweep-report.schema.json".to_owned(),
            absolute_path: repo_root.join("src/semantic/sweep-report.schema.json"),
            required: true,
        },
        Embed {
            internal_name: "schemas/module-manifest.schema.json".to_owned(),
            absolute_path: filament_core_data_semantic_v1.join("module-manifest.schema.json"),
            required: false,
        },
        Embed {
            internal_name: "schemas/filament-core-data/common.schema.json".to_owned(),
            absolute_path: filament_core_data_semantic_v1.join("common.schema.json"),
            required: false,
        },
        Embed {
            internal_name: "schemas/filament-core-data/package-manifest.schema.json".to_owned(),
            absolute_path: filament_core_data_semantic_v1.join("package-manifest.schema.json"),
            required: false,
        },
    ];
    semantic_core_members(&semantic_core_pkg, &mut embeds)?;
    embeds.sort_by(|a, b| a.internal_name.cmp(&b.internal_name));

    for path in [
        repo_root.join("src/semantic"),
        semantic_core_pkg.clone(),
        filament_core_data_pkg.clone(),
    ] {
        println!("cargo::rerun-if-changed={}", path.display());
    }

    let mut generated = String::from("pub(super) static EMBEDDED_FILES: &[(&str, &[u8])] = &[\n");
    for embed in &embeds {
        if !embed.absolute_path.is_file() {
            if embed.required {
                return Err(format!(
                    "quoin-semantic's build.rs expected {} to exist; run `pnpm install` at the \
                     repository root (or `make workflow-assets`) to populate node_modules",
                    embed.absolute_path.display()
                )
                .into());
            }
            println!(
                "cargo::warning=quoin-semantic: {} does not exist yet -- \
                 @agent-ix/filament-core-data has not published schema/semantic/v1/ \
                 (PLAT-887; tracked separately). {} will be unavailable at runtime \
                 (SemanticError::VendoredSchemaUnreadable) until it is published and \
                 `pnpm install` picks it up.",
                embed.absolute_path.display(),
                embed.internal_name
            );
            continue;
        }
        let absolute = embed.absolute_path.to_string_lossy().replace('\\', "\\\\");
        writeln!(
            generated,
            "    ({:?}, include_bytes!({absolute:?})),",
            embed.internal_name
        )?;
    }
    generated.push_str("];\n");
    let output = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| std::io::Error::other("cargo did not set OUT_DIR"))?,
    );
    fs::write(output.join("embedded_contract.rs"), generated)?;
    Ok(())
}
