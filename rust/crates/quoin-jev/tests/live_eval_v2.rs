// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The corpus-v2 experiment runner, live (PLAT-1027, parent PLAT-1024).
//!
//! Runs a list of registered variants over one split of corpus v2 and prints
//! the report: agreement, the best constant predictor and the margin over it,
//! defect / no-defect and per-class recall, ECE, ordering quality for
//! severity, and the confidence-gated coverage/accuracy curve, each broken
//! down by mode and by truth kind. It gates nothing: each experiment ticket
//! (PLAT-1028..1031) pre-registers its own bars in an MP doc before its first
//! live call, and reads them off this report.
//!
//! ```bash
//! cd rust && QUOIN_JEV_VARIANTS=S0,E0,T0,C0 \
//!   cargo test -p quoin-jev --features live-api --test live_eval_v2 -- --nocapture
//! ```
//!
//! | env | meaning |
//! | --- | --- |
//! | `QUOIN_JEV_VARIANTS` | comma-separated variant ids; default every registered one. Each `variant@version` must match its request-digest pin and be within the 5-version dev cap (`eval_v2_support::preflight`) |
//! | `QUOIN_JEV_SPLIT` | `dev` (default) or `heldout` |
//! | `QUOIN_JEV_HELDOUT` | must be `1` for `heldout`; the seal is verified, every variant must be covered by `fixtures/eval-v2/heldout-selection.json`, and the run is logged before any number prints |
//! | `QUOIN_JEV_HELDOUT_RERUN` | a reason; required to run a variant version already on the held-out log |
//! | `QUOIN_JEV_EXTERNAL_CORPUS` / `QUOIN_JEV_EXTERNAL_ROOT` | optional external corpus and its checkouts |
//! | `QUOIN_JEV_CASSETTE` | a cassette file (PLAT-977): answers are recorded there, and re-grading replays them |
//! | `QUOIN_JEV_CASSETTE_MODE` | `record` (default: replay what is on file, call live for the rest) or `replay` (no network, no key) |
//! | `QUOIN_JEV_MODEL` | the pinned model; required with a cassette, and when set every answer must come from it |
//!
//! With no cassette the run goes through `quoin_jev::client::production`,
//! exactly like the sibling live files. A cassette in record mode wraps the
//! same real transport, so the first run costs what an uncached run costs and
//! every later run over the same requests costs nothing.
//!
//! Panics rather than skipping when no key resolves on a live path, and when
//! there are no rows to run: a silent skip is how a suite reports green over
//! a measurement that never ran.

#![cfg(feature = "live-api")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

mod eval_v2_support;
mod gap_semantic_support;

use std::collections::BTreeMap;
use std::sync::Arc;

use typesafe_sdk_client::Client;
use typesafe_sdk_env::{Fixed, Process};
use typesafe_sdk_http::Reqwest;

use eval_v2_support::corpus::{
    self, Excluded, HELDOUT_ENV, HELDOUT_RERUN_ENV, HeldoutRun, HeldoutSpend, Row, Source, Split,
    combined_rows, validate,
};
use eval_v2_support::metrics::render_run;
use eval_v2_support::preflight::{self, RunGate};
use eval_v2_support::variant::{self, REGISTRY, Variant};
use eval_v2_support::variants::soundness;
use quoin_jev::{Cassette, JevErrorCode};

fn env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

fn live_config() -> typesafe_sdk_config::Config {
    quoin_jev::config::resolve(&Process).unwrap_or_else(|error| {
        assert_eq!(
            error.code,
            JevErrorCode::MissingKey,
            "unexpected config failure: {error}"
        );
        panic!(
            "the `live-api` feature is on but no API key resolved. This test does not skip. \
             Set TYPESAFE_API_KEY, or replay a cassette with QUOIN_JEV_CASSETTE_MODE=replay."
        );
    })
}

/// The client: production, a recording cassette over production's transport,
/// or a replaying cassette with no network at all.
fn client(model: Option<&str>) -> Client {
    let Some(path) = env("QUOIN_JEV_CASSETTE") else {
        return quoin_jev::client::production(live_config()).expect("builds a real transport");
    };
    let model = model.unwrap_or_else(|| {
        panic!("QUOIN_JEV_CASSETTE needs QUOIN_JEV_MODEL: a cassette is pinned to one model")
    });
    match env("QUOIN_JEV_CASSETTE_MODE")
        .as_deref()
        .unwrap_or("record")
    {
        "replay" => {
            let config = quoin_jev::config::resolve(&Fixed::new(&[(
                "TYPESAFE_API_KEY",
                "replay-sends-no-request",
            )]))
            .expect("a fixed key resolves");
            let cassette = Cassette::replay(&path, model).unwrap_or_else(|error| panic!("{error}"));
            quoin_jev::client::with_transport(config, Arc::new(cassette))
        }
        "record" => {
            let delegate = Reqwest::new().expect("builds a real transport");
            let cassette = Cassette::record(&path, Arc::new(delegate), model)
                .unwrap_or_else(|error| panic!("{error}"));
            quoin_jev::client::with_transport(live_config(), Arc::new(cassette))
        }
        other => panic!("QUOIN_JEV_CASSETTE_MODE must be `record` or `replay`, not {other:?}"),
    }
}

fn sources() -> Vec<Source> {
    let sources: Vec<Source> = [
        corpus::load_in_repo().unwrap_or_else(|error| panic!("{error}")),
        corpus::load_external_from_env().unwrap_or_else(|error| panic!("{error}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    for source in &sources {
        let problems = validate(&source.file, source.origin);
        assert!(
            problems.is_empty(),
            "{}: {} problem(s):\n  - {}",
            source.path.display(),
            problems.len(),
            problems.join("\n  - ")
        );
    }
    sources
}

/// Provenance: PLAT-1027. Runs the chosen variants over the chosen split and
/// prints the report. Ungated: bars belong to each experiment's MP doc.
#[tokio::test]
async fn tc_1027_run_variants_over_corpus_v2() {
    let variants: Vec<&Variant> = match env("QUOIN_JEV_VARIANTS") {
        Some(ids) => variant::resolve(&ids).unwrap_or_else(|error| panic!("{error}")),
        None => REGISTRY.iter().collect(),
    };
    let split = match env("QUOIN_JEV_SPLIT").as_deref().unwrap_or("dev") {
        "dev" => Split::Dev,
        "heldout" => Split::Heldout,
        other => panic!("QUOIN_JEV_SPLIT must be `dev` or `heldout`, not {other:?}"),
    };
    let sources = sources();
    assert!(
        !sources.is_empty(),
        "no corpus to run: {} does not exist and {} is unset",
        corpus::in_repo_corpus_path().display(),
        corpus::EXTERNAL_CORPUS_ENV
    );
    let labels: Vec<String> = variants.iter().map(|variant| variant.label()).collect();
    let rerun_reason = env(HELDOUT_RERUN_ENV);
    let heldout_flag = env(HELDOUT_ENV);
    let selection = corpus::heldout_selection_path();
    let log = corpus::heldout_log_path();
    let refs: Vec<&Source> = sources.iter().collect();
    let seals = preflight::authorize_run(
        &RunGate {
            split,
            heldout_flag: heldout_flag.as_deref(),
            rerun_reason: rerun_reason.as_deref(),
            selection: &selection,
            log: &log,
        },
        &variants,
        &refs,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let rows: Vec<Row> = combined_rows(&sources, split).unwrap_or_else(|error| panic!("{error}"));
    let excluded: Vec<Excluded> = sources
        .iter()
        .flat_map(|source| source.excluded.iter().cloned())
        .collect();
    assert!(!rows.is_empty(), "no {} rows to run", split.as_str());

    eval_v2_support::variants::intent::require_cassette(
        &variants,
        env("QUOIN_JEV_CASSETTE").as_deref(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let model = env("QUOIN_JEV_MODEL");
    let client = client(model.as_deref());

    // The held-out split is spent the moment the first answer exists, so the
    // spend is begun before any request is sent. A run that stops part way,
    // on a malformed answer or a transport error, is still logged when the
    // guard drops, so the rerun check fires on the next attempt.
    let spend = (split == Split::Heldout).then(|| {
        HeldoutSpend::begin(
            corpus::heldout_log_path(),
            HeldoutRun {
                unix_seconds: 0,
                variants: labels.clone(),
                seals,
                rows: rows.len(),
                models: BTreeMap::new(),
                rerun_reason,
            },
        )
    });
    let output = variant::run(&client, &rows, &variants)
        .await
        .unwrap_or_else(|error| panic!("{error}"));

    // Logged before any number is printed and before any assertion that
    // could stop the run.
    if let Some(spend) = spend {
        spend
            .finish(output.models.clone())
            .unwrap_or_else(|error| panic!("{error}"));
        println!(
            "held-out run recorded in {}; commit it with the report",
            corpus::heldout_log_path().display()
        );
    }
    println!(
        "split {}: {} rows; variants {labels:?}",
        split.as_str(),
        rows.len()
    );
    println!("{}", render_run(&rows, &excluded, &output, &variants));
    print!(
        "{}",
        eval_v2_support::variants::exceeds::render_diagnostics(&rows, &output)
    );
    println!(
        "{}",
        eval_v2_support::variants::intent::render_gated_run(&rows, &output, &variants)
    );
    // Experiment 2 diagnostics (PLAT-1024); informational, written only on request.
    // Dev only: diagnosing on held-out rows would contaminate the seal.
    if let Some(dir) = env(eval_v2_support::exp2::OUT_ENV) {
        assert!(
            split == Split::Dev,
            "{} is dev-only; held-out rows are never diagnosed",
            eval_v2_support::exp2::OUT_ENV
        );
        eval_v2_support::exp2::write(std::path::Path::new(&dir), &rows, &output)
            .unwrap_or_else(|error| panic!("{error}"));
        println!("exp2 diagnostics written to {dir}");
    }
    // MP-243's bars (PLAT-1031); empty unless a K variant ran.
    println!("{}", soundness::render_bars(&rows, &output, &variants));
    println!(
        "{}",
        eval_v2_support::variants::severity::render_bars(&rows, &output, &variants)
    );

    if let Some(model) = &model {
        let others: Vec<&String> = output.models.keys().filter(|seen| *seen != model).collect();
        assert!(
            others.is_empty(),
            "answers came from {others:?}, not the pinned {model}"
        );
    }
}
