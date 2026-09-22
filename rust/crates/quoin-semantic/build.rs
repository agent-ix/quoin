// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Embed Quoin's semantic contract into native releases (quoin#527).
//!
//! Two families of schema make up the contract (FR-070, FR-073): one quoin
//! owns (`src/semantic/sweep-report.schema.json`), and the rest it depends on
//! rather than vendors -- `agent-ix/filament-core-data`'s module-manifest,
//! package-manifest and common schemas (published as
//! `@agent-ix/semantic-schema`), and its semantic-core JSON Schema bundle
//! (published as `@agent-ix/semantic-core`) -- read straight out of the
//! published npm packages, resolved through this repository's own
//! `node_modules` (PLAT-887's de-vendoring; `make workflow-assets` /
//! `pnpm install` populates it, the same mechanism `quoin-cli::flow` uses for
//! `@agent-ix/ix-spec-workflows`). Nothing is copied into this repository's
//! own tree: `include_bytes!` reads the installed package's files directly,
//! at their own paths, and the generated constant below only re-labels each
//! one with the internal name [`crate::contract`]'s path helpers already
//! expect (`schemas/semantic-core/<Type>.json`, and so on) -- a label the
//! compiled binary and [`crate::embedded::materialize_embedded_contract`] use
//! to lay the bytes out in a runtime cache dir, unrelated to the packages'
//! own directory layout.
//!
//! Every embed here is required: a missing file fails the build rather than
//! producing a binary that is silently missing a schema. Both packages are
//! pinned to an exact version rather than a range: a compatible-looking
//! `^x.y.z` resolving to a version whose content differs would otherwise fail
//! [`crate::contract::SEMANTIC_CONTRACT`]'s digest assertions with a
//! confusing red. That digest, not the version string, is the real gate --
//! the pin just avoids a predictable, unhelpful failure at resolve time
//! instead of at the digest check.
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
/// and the absolute path `include_bytes!` reads it from. Every embed is
/// required -- a missing file fails the build rather than shipping a binary
/// silently missing a schema.
struct Embed {
    internal_name: String,
    absolute_path: PathBuf,
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
        });
    }
    embeds.push(Embed {
        internal_name: "schemas/semantic-core/toolchain.json".to_owned(),
        absolute_path: semantic_core_pkg.join("generated/toolchain.json"),
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
    let semantic_schema_pkg = node_modules_agent_ix.join("semantic-schema");
    let semantic_schema_v1 = semantic_schema_pkg.join("semantic/v1");

    let mut embeds = vec![
        Embed {
            internal_name: "sweep-report.schema.json".to_owned(),
            absolute_path: repo_root.join("src/semantic/sweep-report.schema.json"),
        },
        Embed {
            internal_name: "schemas/module-manifest.schema.json".to_owned(),
            absolute_path: semantic_schema_v1.join("module-manifest.schema.json"),
        },
        Embed {
            internal_name: "schemas/filament-core-data/common.schema.json".to_owned(),
            absolute_path: semantic_schema_v1.join("common.schema.json"),
        },
        Embed {
            internal_name: "schemas/filament-core-data/package-manifest.schema.json".to_owned(),
            absolute_path: semantic_schema_v1.join("package-manifest.schema.json"),
        },
    ];
    semantic_core_members(&semantic_core_pkg, &mut embeds)?;
    embeds.sort_by(|a, b| a.internal_name.cmp(&b.internal_name));

    for path in [
        repo_root.join("src/semantic"),
        semantic_core_pkg.clone(),
        semantic_schema_pkg.clone(),
    ] {
        println!("cargo::rerun-if-changed={}", path.display());
    }

    let mut generated = String::from("pub(super) static EMBEDDED_FILES: &[(&str, &[u8])] = &[\n");
    for embed in &embeds {
        if !embed.absolute_path.is_file() {
            return Err(format!(
                "quoin-semantic's build.rs expected {} to exist; run `pnpm install` at the \
                 repository root (or `make workflow-assets`) to populate node_modules",
                embed.absolute_path.display()
            )
            .into());
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
