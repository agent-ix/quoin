// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `make types`: write `src/core/types.ts` and record what it was generated
//! from (FR-097).
//!
//! Modelled on `scripts/refresh-quire-schemas.mjs`, which is this repository's
//! working precedent for a generated-or-vendored artefact under a hash
//! assertion: the tool rewrites both the artefact and the digest recorded
//! against it, in one act, so the two cannot be updated apart. What that script
//! does for the vendored quire schemas, this binary does for the boundary
//! types — the difference being only that the bytes come from a `schemars`
//! render rather than from a pinned git object.
//!
//! This is the ONLY thing that may write `src/core/types.ts` or move the
//! digests in `lib.rs`. Everything else asserts them.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Where the repository root is, unless `--repo` says otherwise.
///
/// `crates/quoin-schemas` → `crates` → `rust` → the repository.
fn default_repo_root() -> PathBuf {
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..3 {
        root = root
            .parent()
            .map_or_else(|| root.clone(), Path::to_path_buf);
    }
    root
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let mut repo = default_repo_root();
    let mut check_only = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--repo" => {
                let Some(value) = args.next() else {
                    eprintln!("--repo needs a path");
                    return ExitCode::from(3);
                };
                repo = PathBuf::from(value);
            }
            // Useful by hand; the gate uses the crate's tests, which fail with
            // the same findings and do not need a built binary.
            "--check" => check_only = true,
            other => {
                eprintln!("usage: quoin-schemas-gen [--repo <path>] [--check]; got {other}");
                return ExitCode::from(3);
            }
        }
    }

    let rendered = match quoin_schemas::typescript_source() {
        Ok(rendered) => rendered,
        Err(refusal) => {
            eprintln!(
                "the boundary schema carries a construct the renderer refuses: {refusal}\n\
                 Add the case to `render.rs` with a test; do NOT widen the type."
            );
            return ExitCode::from(2);
        }
    };
    let types_path = repo.join(quoin_schemas::GENERATED_TYPES_PATH);
    let schema_digest = quoin_schemas::source_schema_digest();
    let content_digest = quoin_schemas::sha256_hex(rendered.as_bytes());

    if check_only {
        let on_disk = match std::fs::read_to_string(&types_path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("{}: {error}", types_path.display());
                return ExitCode::from(4);
            }
        };
        if let Err(finding) = quoin_schemas::check_generated_source(&on_disk) {
            eprintln!("{finding}");
            return ExitCode::from(1);
        }
        println!(
            "{} is generated, current and unmodified",
            types_path.display()
        );
        return ExitCode::SUCCESS;
    }

    if let Err(error) = std::fs::write(&types_path, &rendered) {
        eprintln!("{}: {error}", types_path.display());
        return ExitCode::from(4);
    }

    let lib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let lib = match std::fs::read_to_string(&lib_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("{}: {error}", lib_path.display());
            return ExitCode::from(4);
        }
    };
    let lib = match replace_digest(&lib, "SOURCE_SCHEMA_SHA256", &schema_digest)
        .and_then(|text| replace_digest(&text, "GENERATED_TYPES_SHA256", &content_digest))
    {
        Ok(text) => text,
        Err(message) => {
            eprintln!("{}: {message}", lib_path.display());
            return ExitCode::from(4);
        }
    };
    if let Err(error) = std::fs::write(&lib_path, lib) {
        eprintln!("{}: {error}", lib_path.display());
        return ExitCode::from(4);
    }

    println!("wrote {}", types_path.display());
    println!("  source-schema sha256 {schema_digest}");
    println!("  generated-file sha256 {content_digest}");
    println!("  recorded in {}", lib_path.display());
    ExitCode::SUCCESS
}

/// Replace the 64-hex literal declared for `name` in `lib.rs`.
///
/// Located by the declaration rather than by the old value: the two digests
/// are indistinguishable as strings, so a value-keyed replacement would be
/// ambiguous the moment they coincided — which they do on a fresh checkout of
/// this file's initial state.
fn replace_digest(text: &str, name: &str, value: &str) -> Result<String, String> {
    let declaration = format!("pub const {name}: &str =");
    let start = text
        .find(&declaration)
        .ok_or_else(|| format!("no declaration of `{name}`"))?;
    let rest = text
        .get(start..)
        .ok_or_else(|| format!("no declaration of `{name}`"))?;
    let open = rest
        .find('"')
        .ok_or_else(|| format!("`{name}` declares no string literal"))?;
    let close = rest
        .get(open + 1..)
        .and_then(|tail| tail.find('"'))
        .ok_or_else(|| format!("`{name}`'s string literal is unterminated"))?;
    let absolute_open = start + open + 1;
    let absolute_close = absolute_open + close;
    let mut out = String::with_capacity(text.len());
    out.push_str(text.get(..absolute_open).unwrap_or_default());
    out.push_str(value);
    out.push_str(text.get(absolute_close..).unwrap_or_default());
    Ok(out)
}
