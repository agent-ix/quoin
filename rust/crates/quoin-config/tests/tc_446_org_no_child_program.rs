// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Org resolution names no child program, checked on the module that decides
//! it rather than asserted in prose (FR-025-AC-7).
//!
//! FR-025-AC-7 used to read "Resolution executes no subprocess", and quoin#446
//! made that false of the shipping CLI: resolving an org now calls `quoin-core`
//! once. The criterion was amended rather than dropped, because the property it
//! was written against is still the one that matters — resolution must not shell
//! out to `git`, so it holds on a host with no Git executable and with nothing
//! useful on `PATH` (NFR-004). The amended criterion therefore has two halves,
//! and they are carried in two places:
//!
//! * **The CLI executes exactly one subprocess, and it is `quoin-core`** —
//!   `tests/org-one-subprocess.test.ts`, which replaces every `node:child_process`
//!   entry point with a recorder and reads back what was actually spawned. That
//!   is the file that succeeds `tests/org-no-subprocess.test.ts`, deleted with
//!   `src/org.ts`.
//! * **The library that decides the org spawns nothing at all** — this file.
//!
//! The audit is `engineering_assurance::source_audit`, the same auditor
//! `quoin-core`'s `tc_library_containment.rs` runs, consumed rather than
//! regrown. It is scoped to `src/org.rs` and to the [`RustSourceCapability`]
//! values the criterion is about, and neither of those is a relaxation of that
//! gate: `tc_library_containment` still audits every `quoin-core` library source
//! for every capability, and is untouched.
//!
//! Scoped to `org.rs` because `org.rs` is what FR-025 is a requirement about.
//! Scoped to `ChildProgram` and `Network` because `org.rs` legitimately names
//! [`Filesystem`](RustSourceCapability::Filesystem): `resolve_org` reads
//! `.git/config` from disk for the in-process caller. The *boundary* does not
//! take that path — it calls `resolve_org_from_documents`, which is handed the
//! three documents' content and opens nothing, which is what
//! `quoin-core`'s own containment audit proves about `ops/config.rs` and what
//! `tc_446_org_parity.rs` proves about the two paths agreeing.

use engineering_assurance::source_audit::{
    RustSourceAuditRole, RustSourceCapability, RustSourceFindingCategory, audit_rust_source,
};

/// The capabilities FR-025-AC-7 forbids the resolver: a child program is the
/// `git` invocation the criterion exists to prevent, and a socket is the other
/// way the same answer could arrive from outside the process.
const FORBIDDEN: [RustSourceCapability; 2] = [
    RustSourceCapability::ChildProgram,
    RustSourceCapability::Network,
];

/// The resolver's source, read from the crate this test lives in.
///
/// A report line rather than a panic on failure: the workspace lints
/// `clippy::panic` in tests as well as production, and an unreadable source
/// belongs in the same report as a containment breach rather than aborting it.
fn org_source() -> Result<Vec<u8>, String> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("org.rs");
    std::fs::read(&path).map_err(|error| format!("{}: unreadable ({error})", path.display()))
}

/// Every forbidden capability `source` names, as report lines.
fn offences(source: &[u8], label: &str) -> Vec<String> {
    match audit_rust_source(source, RustSourceAuditRole::ReusableLibrary) {
        Ok(findings) => findings
            .into_iter()
            .filter(|finding| finding.category() == RustSourceFindingCategory::ForbiddenCapability)
            .filter_map(|finding| {
                finding
                    .capability()
                    .filter(|capability| FORBIDDEN.contains(capability))
                    .map(|capability| format!("{label}: {capability:?}"))
            })
            .collect(),
        Err(error) => vec![format!("{label}: {} ({error})", error.code())],
    }
}

#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::{offences, org_source};

    /// Trace: FR-025-AC-7
    /// Provenance: quoin#446
    #[test]
    fn tc_446_070_org_resolution_names_no_child_program() {
        let source = match org_source() {
            Ok(bytes) => bytes,
            Err(report) => {
                assert_eq!(vec![report], Vec::<String>::new(), "src/org.rs");
                return;
            }
        };

        // The population, stated rather than assumed: an audit of the wrong file,
        // or of a file that stopped holding the decision, would otherwise report
        // a clean resolver while the resolver moved elsewhere.
        let text = std::str::from_utf8(&source).unwrap_or_default();
        assert!(
            text.contains("pub fn resolve_org_from_documents")
                && text.contains("pub fn org_from_git_config"),
            "src/org.rs no longer holds org resolution; this audit is pointed at \
             the wrong file rather than reporting a clean one"
        );

        assert_eq!(
            offences(&source, "src/org.rs"),
            Vec::<String>::new(),
            "org resolution acquired a child-program or network capability; \
             FR-025-AC-7 permits exactly one subprocess, the quoin-core call the \
             CLI makes, and nothing below it"
        );
    }

    /// Trace: FR-025-AC-7
    /// Provenance: quoin#446
    #[test]
    fn tc_446_071_the_audit_reports_a_child_program_when_one_is_named() {
        // A control on the check, not evidence for the criterion: the assertion
        // above is worth nothing unless a resolver that did shell out would fail
        // it, and an auditor that silently stopped recognising `std::process`
        // would make it pass forever.
        let shells_out = br#"
            pub fn org_from_git_config(root: &str) -> Option<String> {
                let out = std::process::Command::new("git").arg("config").output().ok()?;
                String::from_utf8(out.stdout).ok()
            }
        "#;

        assert_eq!(
            offences(shells_out, "control"),
            vec!["control: ChildProgram".to_owned()],
            "the auditor no longer recognises a `git` subprocess, so the \
             assertion in tc_446_070 proves nothing"
        );
    }
}
