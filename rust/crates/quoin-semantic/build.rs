// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Embed Quoin's vendored semantic contract into native releases.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

fn collect(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = fs::read_dir(directory)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect(root, &path, files)?;
        } else {
            files.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| std::io::Error::other("cargo did not set CARGO_MANIFEST_DIR"))?,
    );
    let root = manifest.join("../../../src/semantic");
    println!("cargo::rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect(&root, &root, &mut files)?;
    files.sort();

    let mut generated = String::from("pub(super) static EMBEDDED_FILES: &[(&str, &[u8])] = &[\n");
    for relative in files {
        let relative = relative.to_string_lossy().replace('\\', "/");
        let absolute = root.join(&relative).to_string_lossy().replace('\\', "\\\\");
        writeln!(
            generated,
            "    ({relative:?}, include_bytes!({absolute:?})),"
        )?;
    }
    generated.push_str("];\n");
    let output = PathBuf::from(
        env::var_os("OUT_DIR").ok_or_else(|| std::io::Error::other("cargo did not set OUT_DIR"))?,
    );
    fs::write(output.join("embedded_contract.rs"), generated)?;
    Ok(())
}
