// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Producers for the NFR percentage metrics bound to enumerated populations
//! (PLAT-1401). Each producer enumerates its population from the code under
//! measurement, probes every member against the built binary, and states the
//! ids it examined and the ids that matched.

#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

pub(crate) mod commands;
pub(crate) mod manifests;
pub(crate) mod span_breadth;

use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use quoin_measurement::json_bridge::from_serde;
use quoin_measurement::plans::{PlanLoadOptions, load_measurement_plans};
use quoin_measurement::source::MemoryMeasurement;
use quoin_measurement::types::plan::MeasurementPlan;
use quoin_measurement::validate;
use quoin_measurement::verify::{OrderSource, Ranked, TamperFacts, Verdict, verify};
use serde_json::{Value, json};

/// What one producer found: the ids it examined and the ids that matched.
pub(crate) struct Probed {
    pub(crate) examined: BTreeSet<String>,
    pub(crate) matched: BTreeSet<String>,
}

/// The longest any spawned `quoin` may run before the probe is a failure.
const TIMEOUT: Duration = Duration::from_secs(120);

/// The built `quoin` binary, run with an isolated home and a deadline. A
/// process that outlives the deadline is killed and the probe panics: a hang
/// is never read as a refusal.
pub(crate) fn quoin(
    home: &Path,
    arguments: &[&str],
    configure: impl FnOnce(&mut Command),
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_quoin"));
    command
        .args(arguments)
        .env("IX_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure(&mut command);
    let mut child = command.spawn().expect("the native quoin binary spawns");
    let drain = |stream: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut stream) = stream {
                let _ = stream.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = drain(child.stdout.take().map(|s| Box::new(s) as _));
    let stderr = drain(child.stderr.take().map(|s| Box::new(s) as _));
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child can be polled") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("`quoin {arguments:?}` ran past {TIMEOUT:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: stdout.join().expect("stdout drains"),
        stderr: stderr.join().expect("stderr drains"),
    }
}

pub(crate) fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

pub(crate) fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// The authored plan `id` (for example `MP-250`), loaded from `spec/assurance`.
pub(crate) fn plan(id: &str) -> MeasurementPlan {
    let assurance = repository().join("spec/assurance");
    let file = std::fs::read_dir(&assurance)
        .expect("spec/assurance is readable")
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.starts_with(&format!("{id}-")))
        .unwrap_or_else(|| panic!("no plan file for {id}"));
    let document = std::fs::read_to_string(assurance.join(&file)).expect("the plan is readable");
    let source = MemoryMeasurement::new().with_document(format!("spec/assurance/{file}"), document);
    load_measurement_plans(&source, PlanLoadOptions::default())
        .expect("the authored plan loads")
        .remove(0)
}

/// Check `probed` against the authored plan `id`: the plan's declared set is
/// exactly what the code enumerated, the observation is admitted at intake,
/// and the checker accepts it. Keeps the admitted collection when
/// `NFR_COLLECTION_OUT` names a directory.
pub(crate) fn assert_accepted(id: &str, probed: &Probed) {
    let verdict = verdict(id, probed);
    assert_eq!(
        (verdict.verdict, &verdict.reasons),
        (Verdict::Accept, &vec![]),
        "{id}: {:?}",
        verdict.findings
    );
}

/// Check `probed` against the authored plan `id` as [`assert_accepted`] does,
/// and return the checker's verdict without requiring it to be an accept: a
/// measured value below the plan's bar is a result, an unstated population is
/// not.
pub(crate) fn verdict(id: &str, probed: &Probed) -> quoin_measurement::verify::MeasurementVerdict {
    let plan = plan(id);
    let declared: BTreeSet<String> = plan
        .statistical_design
        .as_ref()
        .and_then(|design| design.population_set.as_ref())
        .expect("the plan declares a population set")
        .iter()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        declared, probed.examined,
        "{id}: the declared set differs from the one the code enumerates"
    );

    let (examined, matched) = (probed.examined.len(), probed.matched.len());
    #[allow(clippy::cast_precision_loss, reason = "a handful of ids")]
    let value = matched as f64 / examined as f64;
    let candidate = json!({
        "schemaVersion": 2,
        "collectionId": format!("population-{id}"),
        "subject": "quoin",
        "scope": { "ids": examined },
        "toolIdentity": "quoin-cli population probes",
        "toolVersion": format!("quoin-cli {} (engine tc_3003)", env!("CARGO_PKG_VERSION")),
        "timestamp": "2026-11-11T00:00:00.000Z",
        "environment": { "runner": "cargo test" },
        "verificationStack": {
            "schemaVersion": "verification-stack-attestation-v1",
            "buildProfile": "debug",
            "capabilities": ["population.probe"],
            "artifacts": [plan.path],
        },
        "observations": [observation(&plan, value, probed)],
        "rawEvidence": { "examined": probed.examined, "matched": probed.matched },
    });
    let admitted = validate::measurement_collection(
        &from_serde(&candidate).expect("the candidate crosses the bridge"),
        std::slice::from_ref(&plan),
    )
    .unwrap_or_else(|error| panic!("{id}: intake refused the observation: {error}"));
    if let Some(directory) = std::env::var_os("NFR_COLLECTION_OUT") {
        std::fs::create_dir_all(&directory).expect("the output directory is creatable");
        std::fs::write(
            Path::new(&directory).join(format!("{id}.json")),
            serde_json::to_string_pretty(&candidate).expect("the candidate serialises"),
        )
        .expect("the collection is writable");
    }

    verify(
        &plan,
        &[Ranked::new(&admitted, Some(0))],
        TamperFacts::default(),
        OrderSource::CallerSupplied,
        None,
    )
}

fn observation(plan: &MeasurementPlan, value: f64, probed: &Probed) -> Value {
    json!({
        "metric": plan.metric.as_str(),
        "planId": plan.id.as_str(),
        "definitionVersion": plan.definition_version.as_str(),
        "state": "measured",
        "value": value,
        "unit": "fraction",
        "shape": "ratio",
        "population": {
            "examined": probed.examined.len(),
            "matched": probed.matched.len(),
            "complete": true,
            "repetitions": 1,
            "examined_ids": probed.examined,
            "matched_ids": probed.matched,
        },
    })
}
