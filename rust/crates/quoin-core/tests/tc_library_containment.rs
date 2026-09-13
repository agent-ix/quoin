// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The library half of `quoin-core` holds no host capability, checked by the
//! shared Engineering Assurance auditor rather than by a local one.
//!
//! `.claude/skills/rust-style/SKILL.md` states the rule in prose: "`main.rs`
//! does four things — argv, stdin, dispatch, two writes and an exit — and must
//! keep doing only four. Everything decidable lives in the library so it is
//! unit-testable without spawning a process." Prose is not a gate. A single
//! `std::fs::read_to_string` added to `ops/` would pass every existing test,
//! pass clippy, and quietly make the boundary untestable without a filesystem.
//!
//! The audit is `engineering_assurance::source_audit`, consumed, not regrown
//! (quire-research `implementation-language-policy.md`, "Shared tooling is
//! consumed, not regrown"). quoin does not own a second syn-based Rust source
//! auditor; when this one does not fit, the answer is a ticket in
//! agent-ix/engineering-assurance.

use std::path::{Path, PathBuf};

use engineering_assurance::source_audit::{
    RustSourceAuditRole, RustSourceFindingCategory, audit_rust_source,
};

/// The one file in this crate allowed to hold a host capability: the I/O
/// shell. Everything else under `src/` is the library half.
const IO_SHELL: &str = "main.rs";

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Collect every `.rs` file under `dir`, so a module added tomorrow is audited
/// without anyone remembering to extend a list. A hard-coded list is the
/// failure mode this walk exists to avoid.
fn library_sources(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            library_sources(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && path.file_name().is_some_and(|name| name != IO_SHELL)
        {
            out.push(path);
        }
    }
    Ok(())
}

/// Trace: FR-096, NFR-024
/// Provenance: quoin#373
#[test]
fn tc_373_the_library_half_names_no_host_capability() {
    let root = crate_root();
    let mut sources = Vec::new();
    let walk = library_sources(&root.join("src"), &mut sources);
    let mut offences = match walk {
        Ok(()) => Vec::new(),
        Err(error) => vec![format!("src/: unwalkable ({error})")],
    };
    assert!(
        sources.len() >= 5,
        "found only {} library sources; the walk is broken, not the crate",
        sources.len()
    );

    for path in &sources {
        let relative = path.strip_prefix(&root).unwrap_or(path).display();
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            // Not a panic: the workspace lints `clippy::panic` in tests as
            // well as production, and an unreadable source belongs in the same
            // report as a containment breach rather than aborting it.
            Err(error) => {
                offences.push(format!("{relative}: unreadable ({error})"));
                continue;
            }
        };

        let findings = match audit_rust_source(&bytes, RustSourceAuditRole::ReusableLibrary) {
            Ok(findings) => findings,
            Err(error) => {
                offences.push(format!("{relative}: {} ({error})", error.code()));
                continue;
            }
        };

        for finding in findings {
            if finding.category() == RustSourceFindingCategory::ForbiddenCapability {
                offences.push(format!("{relative}: {:?}", finding.capability()));
            }
        }
    }

    assert_eq!(
        offences,
        Vec::<String>::new(),
        "the library half acquired a host capability; move it into main.rs or \
         state why the boundary now needs it"
    );
}
