// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! The Rust half of the cutover gate.
//!
//! ```text
//! node --loader ts-node/esm oracle/capture-store-oracle.mjs --out /tmp/o.json <repo>...
//! quoin-store-replay --oracle /tmp/o.json <repo>...
//! ```
//!
//! Exit status is the gate: `0` only when the two implementations agreed on
//! every digest and every sealed record verified against itself. Anything else
//! exits non-zero and prints what disagreed. The tool never "adjusts" a
//! mismatch; reconciling one is a decision a person makes with the finding in
//! front of them.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use quoin_store::replay::{Oracle, ReplayReport, replay};

fn main() -> ExitCode {
    let mut oracle_path: Option<PathBuf> = None;
    let mut repositories: Vec<PathBuf> = Vec::new();
    let mut verbose = false;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--oracle" => {
                if let Some(value) = arguments.next() {
                    oracle_path = Some(PathBuf::from(value));
                } else {
                    eprintln!("--oracle needs a path");
                    return ExitCode::from(2);
                }
            }
            "--verbose" => verbose = true,
            "--help" | "-h" => {
                println!(
                    "usage: quoin-store-replay [--oracle <capture.json>] [--verbose] <repo>..."
                );
                return ExitCode::SUCCESS;
            }
            other if other.starts_with("--") => {
                eprintln!("unknown option {other}");
                return ExitCode::from(2);
            }
            other => repositories.push(PathBuf::from(other)),
        }
    }
    if repositories.is_empty() {
        eprintln!("usage: quoin-store-replay [--oracle <capture.json>] [--verbose] <repo>...");
        return ExitCode::from(2);
    }

    let oracle = match oracle_path.as_ref().map(load_oracle) {
        None => None,
        Some(Ok(oracle)) => Some(oracle),
        Some(Err(message)) => {
            eprintln!("{message}");
            return ExitCode::from(2);
        }
    };

    let report = match replay(&repositories, oracle.as_ref()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("replay failed: {error} ({})", error.code());
            return ExitCode::from(2);
        }
    };

    print_report(&report, verbose);
    if report.gate_passes() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn load_oracle(path: &PathBuf) -> Result<Oracle, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    Oracle::from_ndjson(&bytes)
        .map_err(|error| format!("cannot read the oracle capture: {error} ({})", error.code()))
}

fn print_report(report: &ReplayReport, verbose: bool) {
    let compared = report.oracle_supplied;
    println!("stores replayed            : {}", report.repositories.len());
    println!("store files found          : {}", report.files_found);
    println!("store files parsed         : {}", report.files_parsed);
    println!("digests replayed           : {}", report.digests_replayed);
    println!(
        "  of which raw outputs     : {}",
        report.raw_outputs_digested
    );
    println!(
        "digest mismatches          : {}{}",
        report.divergences.len(),
        if compared {
            ""
        } else {
            "  (NO ORACLE SUPPLIED — this is not the gate)"
        }
    );
    println!(
        "sealed records verified    : {}",
        report.sealed_records_verified
    );
    println!(
        "retained outputs verified  : {}",
        report.retained_outputs_verified
    );
    println!(
        "round-trip byte-identical  : {} of {}",
        report.round_trip_identical, report.files_parsed
    );
    println!(
        "files rust refused to read : {}",
        report.parse_refusals.len()
    );
    println!(
        "files with __proto__       : {}",
        report.proto_member_files.len()
    );
    println!(
        "foreign sha256 references  : {}",
        report.foreign_sha256_references
    );
    println!(
        "store integrity findings   : {}",
        report.integrity_findings.len()
    );
    println!(
        "COMPARED POPULATION        : {} of {} store entities ({} files + {} raw outputs)",
        report.oracle_comparisons_performed,
        report.store_entities_walked(),
        report.files_found,
        report.raw_outputs_digested
    );
    println!(
        "oracle entries unmatched   : {}",
        report.oracle_entries_unmatched.len()
    );
    println!(
        "store entries unmatched    : {}",
        report.store_entries_unmatched.len()
    );

    for divergence in &report.divergences {
        println!(
            "MISMATCH [{}] {}\n  typescript: {}\n  rust      : {}",
            divergence.kind.as_str(),
            divergence.key,
            divergence.typescript,
            divergence.rust
        );
    }
    for finding in &report.integrity_findings {
        println!("INTEGRITY {}: {}", finding.path.display(), finding.detail);
    }
    for path in &report.proto_member_files {
        println!("PROTO-MEMBER {}", path.display());
    }
    for key in &report.oracle_entries_unmatched {
        println!("ORACLE-ONLY {key}  (the oracle has this entry; this side never walked it)");
    }
    for key in &report.store_entries_unmatched {
        println!("STORE-ONLY {key}  (walked here; the oracle capture has no entry for it)");
    }
    if !compared {
        println!("NO ORACLE SUPPLIED — nothing was compared, so there is no gate to pass");
    }
    if verbose {
        for finding in &report.parse_refusals {
            println!("REFUSED {}: {}", finding.path.display(), finding.detail);
        }
        for path in &report.round_trip_divergent {
            println!("NOT-CANONICAL-ON-DISK {}", path.display());
        }
    }

    println!(
        "\nGATE: {}",
        if report.gate_passes() { "PASS" } else { "FAIL" }
    );
}
