// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `quoin-disk-findings <repo>` — the DISK side of the snapshot differential
//! (quoin#412, review FND-001).
//!
//! A dev-only probe, in the dev-only crate, for one measurement that no other
//! instrument in this repository can take.
//!
//! `quoin validate` reaches [`quoin_validators::inspect_empty_gates_in`] over a
//! [`quoin_validators::MemoryRepo`] built from `src/core/snapshot.ts`'s walk.
//! The same analysis over [`quoin_validators::DiskRepo`] must answer the same
//! thing for the same tree — that equivalence IS the superset property the
//! cutover rests on, on both of its axes at once: which directories the walk
//! skips, and which file names it decides could matter.
//!
//! Nothing could measure it before. `tc_377_019` runs `DiskRepo` over a
//! materialised corpus and `tc_412_the_boundary_reproduces_every_captured_
//! typescript_verdict` runs `MemoryRepo` over the same corpus, and BOTH bypass
//! `repoSnapshot`: the one component whose narrowing silently drops a finding
//! was on neither path. The reviewer of #448 demonstrated the hole by adding
//! `"third_party"` to the TypeScript `EXCLUDED` list — `quoin validate` went
//! from 1 finding to 0 on a tree with a wired empty gate under `third_party/`,
//! and the entire suite, `make rust-gate` included, stayed green.
//!
//! `tests/core-snapshot-differential.test.ts` is the measurement; this binary
//! is the oracle it compares against. It takes a repository path — which is
//! why it cannot live in `quoin-core`, whose library half is audited to name no
//! host capability (`tc_373_the_library_half_names_no_host_capability`) — and
//! writes the same `{"findings": [...]}` payload `validators.run` writes, in
//! canonical JSON, so the two documents are comparable as bytes.
//!
//! It is NOT a boundary operation and is not routed by `dispatch`: it is built
//! by `cargo build --workspace` and used by one test. `quoin-difftest` is its
//! home because that crate is already "dev-only, and exists to compare two
//! answers to one question".

use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [repo] = args.as_slice() else {
        eprintln!("usage: quoin-disk-findings <repository-root>");
        return ExitCode::from(3);
    };

    let findings = match quoin_validators::inspect_empty_gates(Path::new(repo)) {
        Ok(findings) => findings,
        Err(error) => {
            eprintln!("{}: {error}", error.code().as_str());
            return ExitCode::from(2);
        }
    };

    let payload = serde_json::json!({ "findings": findings });
    match quoin_core::protocol::canonical_json(&payload) {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(4)
        }
    }
}
