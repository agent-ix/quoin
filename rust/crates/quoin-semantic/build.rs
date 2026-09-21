// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Embed Quoin's semantic contract into native releases (quoin#527).
//!
//! Three families of schema make up the contract (FR-070, FR-073): one quoin
//! owns (`src/semantic/sweep-report.schema.json`), and two it depends on
//! rather than vendors -- `agent-ix/filament-core-service`'s module-manifest
//! schema and `agent-ix/filament-core-data`'s package-manifest/common schemas
//! and semantic-core JSON Schema bundle, read straight out of the
//! `external/filament-core-service` and `external/filament-core-data` git
//! submodules pinned in this repository. Nothing under `external/` is copied
//! into this repository's own tree at any point: `include_bytes!` reads the
//! submodule's checked-out files directly, at their own paths, and the
//! generated constant below only re-labels each one with the internal name
//! [`crate::contract`]'s path helpers already expect
//! (`schemas/semantic-core/<Type>.json`, and so on) -- a label the compiled
//! binary and [`crate::embedded::materialize_embedded_contract`] use to lay
//! the bytes out in a runtime cache dir, unrelated to the submodules' own
//! directory layout.
//!
//! Explicit files, not a recursive walk: a walk over `external/` would embed
//! two entire third-party repositories (tests, tooling, unrelated packages)
//! into the quoin binary. The semantic-core bundle's member files are the one
//! exception, discovered by directory listing rather than named one by one,
//! because that list is exactly what
//! [`crate::contract::semantic_core_bundle_digest`] already computes the same
//! way -- listing it a second time here, by hand, would be the two
//! sources of truth this workspace's own idiom doc warns against.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// One embedded file: the internal name the runtime cache lays it out under,
/// and the absolute path `include_bytes!` reads it from.
struct Embed {
    internal_name: String,
    absolute_path: PathBuf,
}

fn semantic_core_members(
    filament_core_data: &Path,
    embeds: &mut Vec<Embed>,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_schema_dir = filament_core_data.join("packages/semantic-core/generated/json-schema");
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
        absolute_path: filament_core_data.join("packages/semantic-core/generated/toolchain.json"),
    });
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| std::io::Error::other("cargo did not set CARGO_MANIFEST_DIR"))?,
    );
    let repo_root = manifest.join("../../..");
    let filament_core_service = repo_root.join("external/filament-core-service");
    let filament_core_data = repo_root.join("external/filament-core-data");

    let mut embeds = vec![
        Embed {
            internal_name: "sweep-report.schema.json".to_owned(),
            absolute_path: repo_root.join("src/semantic/sweep-report.schema.json"),
        },
        Embed {
            internal_name: "schemas/module-manifest.schema.json".to_owned(),
            absolute_path: filament_core_service
                .join("filament_core_service/schemas/module-manifest.schema.json"),
        },
        Embed {
            internal_name: "schemas/filament-core-data/common.schema.json".to_owned(),
            absolute_path: filament_core_data.join("schema/semantic/v1/common.schema.json"),
        },
        Embed {
            internal_name: "schemas/filament-core-data/package-manifest.schema.json".to_owned(),
            absolute_path: filament_core_data
                .join("schema/semantic/v1/package-manifest.schema.json"),
        },
    ];
    semantic_core_members(&filament_core_data, &mut embeds)?;
    embeds.sort_by(|a, b| a.internal_name.cmp(&b.internal_name));

    for path in [
        repo_root.join("src/semantic"),
        filament_core_service.clone(),
        filament_core_data.clone(),
    ] {
        println!("cargo::rerun-if-changed={}", path.display());
    }

    let mut generated = String::from("pub(super) static EMBEDDED_FILES: &[(&str, &[u8])] = &[\n");
    for embed in &embeds {
        if !embed.absolute_path.is_file() {
            return Err(format!(
                "quoin-semantic's build.rs expected {} to exist; run `git submodule update \
                 --init external/filament-core-data external/filament-core-service`",
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
