// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `ConfigService` behaviours the org-resolution goldens do not reach:
//! writes, incident recording, doctor verdicts and the read bound (quoin#381).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::PathBuf;

use quoin_config::paths::FixedEnvironment;
use quoin_config::service::{DoctorStatus, IncidentKind, MAX_CONFIG_FILE_BYTES};
use quoin_config::{ConfigErrorCode, ConfigService, IncidentLog, QuoinConfig, RuntimeContext};

struct Fixture {
    _root: tempfile::TempDir,
    env: FixedEnvironment,
    ctx: RuntimeContext,
    user_config: PathBuf,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().expect("temp dir");
    let xdg = root.path().join("xdg");
    let env = FixedEnvironment::new().with_var("XDG_CONFIG_HOME", xdg.to_string_lossy());
    Fixture {
        user_config: xdg.join("ix").join("config.d").join("quoin.yaml"),
        _root: root,
        env,
        ctx: RuntimeContext::new(),
    }
}

fn service(f: &Fixture) -> ConfigService<'_, QuoinConfig> {
    ConfigService::for_plugin(&f.env, &f.ctx)
}

/// Trace: FR-027-AC-1
#[test]
fn tc_381_120_set_key_writes_only_the_named_key() {
    let f = fixture();
    let svc = service(&f);
    let mut log = IncidentLog::new();
    let written = svc.set_key("org", "acme", &mut log).expect("set succeeds");
    assert_eq!(
        written.org.as_ref().map(quoin_config::OrgName::as_str),
        Some("acme")
    );

    let text = fs::read_to_string(&f.user_config).expect("file written");
    // ix-cli-core's header, byte for byte: both implementations write this
    // same file, and a header of our own would churn on alternate writes.
    assert!(
        text.starts_with("# Managed by ix — `ix config edit` to modify safely.\n"),
        "header: {text:?}"
    );
    assert!(text.contains("org: acme"), "body: {text:?}");

    // Re-read through the service, not through the file, so the assertion
    // fails if layering regresses rather than only if the bytes change.
    assert_eq!(
        svc.get(&mut log)
            .value
            .org
            .as_ref()
            .map(quoin_config::OrgName::as_str),
        Some("acme")
    );
}

/// Trace: FR-027-AC-6
#[test]
fn tc_381_121_set_refuses_an_unknown_key_and_writes_nothing() {
    let f = fixture();
    let svc = service(&f);
    let mut log = IncidentLog::new();
    let err = svc
        .set_key("bogus", "x", &mut log)
        .expect_err("the strict schema declares no `bogus`");
    assert_eq!(err.code(), ConfigErrorCode::UnknownConfigKey);
    assert!(
        !f.user_config.exists(),
        "a refused set must not create the file"
    );
}

/// Trace: FR-027-AC-6
#[test]
fn tc_381_122_set_refuses_a_value_the_schema_rejects() {
    let f = fixture();
    let svc = service(&f);
    let mut log = IncidentLog::new();
    let err = svc
        .set_key("org", "", &mut log)
        .expect_err("an empty org fails `min(1)`");
    assert_eq!(err.code(), ConfigErrorCode::ConfigSchema);
    assert!(!f.user_config.exists());
}

/// Trace: FR-027-AC-5
#[test]
fn tc_381_123_get_is_total_and_records_one_incident_per_failure_class() {
    let f = fixture();
    let svc = service(&f);
    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");

    for (label, body, expected) in [
        (
            "unparsable yaml",
            "org: [unterminated\n",
            IncidentKind::Parse,
        ),
        ("not a mapping", "- a\n- b\n", IncidentKind::Parse),
        (
            "unknown key",
            "org: stored\nbogus: 1\n",
            IncidentKind::Schema,
        ),
        ("null org", "org:\n", IncidentKind::Schema),
    ] {
        fs::write(&f.user_config, body).expect("write config");
        let mut log = IncidentLog::new();
        let resolved = svc.get(&mut log);
        assert_eq!(resolved.value, QuoinConfig::default(), "case {label}");
        assert!(resolved.degraded, "case {label} must report degradation");
        let kinds: Vec<IncidentKind> = log.incidents().iter().map(|i| i.kind).collect();
        assert!(
            kinds.contains(&expected),
            "case {label}: expected a {expected} incident, got {kinds:?}"
        );
    }
}

/// Trace: FR-027-AC-5
#[test]
fn tc_381_124_a_config_file_over_the_read_bound_is_an_io_incident() {
    let f = fixture();
    let svc = service(&f);
    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");
    let oversized = "#".repeat(usize::try_from(MAX_CONFIG_FILE_BYTES).expect("fits") + 1);
    fs::write(&f.user_config, oversized).expect("write oversized config");

    let mut log = IncidentLog::new();
    let resolved = svc.get(&mut log);
    assert_eq!(resolved.value, QuoinConfig::default());
    assert_eq!(
        log.incidents().first().map(|i| i.kind),
        Some(IncidentKind::Io),
        "an oversized file must be refused as I/O, not parsed"
    );
}

/// Trace: FR-027-AC-5
#[test]
fn tc_381_125_a_write_replaces_an_unparsable_file_and_reset_clears_it() {
    let f = fixture();
    let svc = service(&f);
    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");
    fs::write(&f.user_config, "org: [unterminated\n").expect("write broken config");

    let mut log = IncidentLog::new();
    svc.get(&mut log);
    assert!(!log.is_empty(), "the broken read must record an incident");

    // The oracle's `set` wraps its read in `try { … } catch { t = {} }`, so an
    // unparsable file is replaced rather than reported. A config so broken it
    // cannot be parsed must not make the one command that repairs it fail.
    let written = svc
        .set_key("org", "acme", &mut log)
        .expect("set replaces an unparsable file, as the oracle does");
    assert_eq!(
        written.org.as_ref().map(quoin_config::OrgName::as_str),
        Some("acme")
    );
    assert!(log.is_empty(), "a successful write clears incidents");
    assert!(
        !fs::read_to_string(&f.user_config)
            .expect("file written")
            .contains("[unterminated"),
        "the unparsable bytes are gone"
    );

    // `reset` still removes the file, and a fresh write still succeeds.
    svc.reset(&mut log).expect("reset removes the file");
    assert!(log.is_empty(), "reset clears this plugin's incidents");
    svc.get(&mut log);
    svc.set_key("org", "acme", &mut log).expect("set succeeds");
    assert!(log.is_empty(), "a successful write clears incidents");
    assert_eq!(
        svc.get(&mut log)
            .value
            .org
            .as_ref()
            .map(quoin_config::OrgName::as_str),
        Some("acme")
    );
}

/// Trace: FR-027-AC-5
#[test]
fn tc_381_126_doctor_reports_valid_invalid_and_key_counts() {
    let f = fixture();
    let svc = service(&f);
    assert_eq!(
        svc.doctor().status,
        DoctorStatus::Valid { key_count: 0 },
        "an absent file validates as an empty config"
    );

    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");
    fs::write(&f.user_config, "org: acme\n").expect("write config");
    assert_eq!(svc.doctor().status, DoctorStatus::Valid { key_count: 1 });

    // Every unknown key at once, not just the first. `deny_unknown_fields`
    // stops at one; zod's `.strict()` reports them all, and so must doctor —
    // otherwise fixing three typos takes three runs.
    fs::write(
        &f.user_config,
        "org: acme\nbogus: 1\nalso-bogus: 2\nstill-bogus: 3\n",
    )
    .expect("write config");
    match svc.doctor().status {
        DoctorStatus::Invalid { issues } => {
            let mut named: Vec<&str> = issues.iter().map(|i| i.key_path.as_str()).collect();
            named.sort_unstable();
            assert_eq!(named, ["also-bogus", "bogus", "still-bogus"]);
        }
        other => panic!("expected Invalid, got {other:?}"),
    }
}

/// Trace: FR-027-AC-9
#[test]
fn tc_381_127_project_layer_beats_the_user_layer_and_writes_stay_user_level() {
    let root = tempfile::tempdir().expect("temp dir");
    let xdg = root.path().join("xdg");
    let project_root = root.path().join("repo").join(".ix");
    fs::create_dir_all(xdg.join("ix").join("config.d")).expect("user dir");
    fs::create_dir_all(project_root.join("config.d")).expect("project dir");
    fs::write(
        xdg.join("ix").join("config.d").join("quoin.yaml"),
        "org: user-level\n",
    )
    .expect("user config");
    fs::write(
        project_root.join("config.d").join("quoin.yaml"),
        "org: project-level\n",
    )
    .expect("project config");

    let env = FixedEnvironment::new().with_var("XDG_CONFIG_HOME", xdg.to_string_lossy());
    let ctx = RuntimeContext::new().with_project_config_root(&project_root);
    let svc = ConfigService::<QuoinConfig>::for_plugin(&env, &ctx);
    let mut log = IncidentLog::new();
    assert_eq!(
        svc.get(&mut log)
            .value
            .org
            .as_ref()
            .map(quoin_config::OrgName::as_str),
        Some("project-level")
    );

    svc.set_key("org", "written", &mut log).expect("set");
    let project_text =
        fs::read_to_string(project_root.join("config.d").join("quoin.yaml")).expect("project file");
    assert!(
        project_text.contains("project-level"),
        "a write must never touch the project layer, got: {project_text:?}"
    );
    let user_text =
        fs::read_to_string(xdg.join("ix").join("config.d").join("quoin.yaml")).expect("user file");
    assert!(user_text.contains("org: written"), "got: {user_text:?}");
}

/// Trace: FR-027-AC-3
#[test]
fn tc_381_128_env_binding_layers_over_both_files() {
    let f = fixture();
    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");
    fs::write(&f.user_config, "org: stored\n").expect("write config");

    let env = FixedEnvironment::new()
        .with_var(
            "XDG_CONFIG_HOME",
            f.user_config
                .parent()
                .and_then(std::path::Path::parent)
                .and_then(std::path::Path::parent)
                .expect("xdg root")
                .to_string_lossy(),
        )
        .with_var("QUOIN_ORG", "from-env");
    let svc = ConfigService::<QuoinConfig>::for_plugin(&env, &f.ctx);
    let mut log = IncidentLog::new();
    assert_eq!(
        svc.get(&mut log)
            .value
            .org
            .as_ref()
            .map(quoin_config::OrgName::as_str),
        Some("from-env")
    );
}

/// Trace: FR-027-AC-5
#[test]
fn tc_381_129_writing_through_a_symlink_is_refused() {
    let f = fixture();
    fs::create_dir_all(f.user_config.parent().expect("has a parent")).expect("config dir");
    let decoy = f
        .user_config
        .parent()
        .expect("has a parent")
        .join("decoy.yaml");
    fs::write(&decoy, "org: victim\n").expect("decoy");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&decoy, &f.user_config).expect("symlink");
    #[cfg(not(unix))]
    return;

    let svc = service(&f);
    let mut log = IncidentLog::new();
    let err = svc
        .set_key("org", "attacker", &mut log)
        .expect_err("a symlinked config target must be refused");
    assert_eq!(err.code(), ConfigErrorCode::ConfigSymlinkRefused);
    assert_eq!(
        fs::read_to_string(&decoy).expect("decoy still readable"),
        "org: victim\n"
    );
}

/// Trace: FR-027-AC-8
#[test]
fn tc_381_130_get_key_reports_declared_unset_and_refuses_undeclared() {
    let f = fixture();
    let svc = service(&f);
    let mut log = IncidentLog::new();

    assert_eq!(
        svc.get_key("org", &mut log).expect("org is declared"),
        None,
        "a declared but unset key reads as unset, not as an error"
    );

    svc.set_key("org", "acme", &mut log).expect("set");
    assert_eq!(
        svc.get_key("org", &mut log)
            .expect("org is declared")
            .and_then(|v| v.as_str().map(ToOwned::to_owned)),
        Some("acme".to_owned())
    );

    assert_eq!(
        svc.get_key("bogus", &mut log)
            .expect_err("the strict schema declares no `bogus`")
            .code(),
        ConfigErrorCode::UnknownConfigKey
    );
}

/// A schema declaring a [`KeyKind::Complex`] key.
///
/// `QuoinConfig` declares only scalars, so nothing in the shipped schema
/// reaches the complex branch of `set_key`. That branch converts a parsed
/// `serde_json::Value` into YAML, and the conversion it must NOT use is
/// `serde_yaml_ng::to_value`: with `serde_json/arbitrary_precision` enabled
/// anywhere in the workspace — it is a global, unifying feature, so one
/// dependency edge turns it on for every crate — a number leaves the serde
/// data model as the private marker map `{"$serde_json::private::Number": "7"}`
/// and that is what a non-serde_json serializer writes. See quoin#442.
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
struct ComplexSchema;

impl quoin_config::PluginConfigSchema for ComplexSchema {
    fn plugin_id() -> quoin_config::PluginId {
        quoin_config::PluginId::new("quoin").expect("legal plugin id")
    }

    fn env_bindings() -> &'static [quoin_config::schema::EnvBinding] {
        &[]
    }

    fn validate(_value: &serde_yaml_ng::Value) -> Result<Self, Vec<quoin_config::ConfigIssue>> {
        Ok(Self)
    }

    fn key_kind(key_path: &str) -> quoin_config::schema::KeyKind {
        match key_path {
            "limits" => quoin_config::schema::KeyKind::Complex,
            _ => quoin_config::schema::KeyKind::Unknown,
        }
    }

    fn json_schema() -> schemars::Schema {
        QuoinConfig::json_schema()
    }
}

/// Trace: FR-027-AC-1
#[test]
fn tc_381_131_set_key_writes_a_complex_value_as_yaml_not_as_a_serde_marker() {
    let f = fixture();
    let svc: ConfigService<'_, ComplexSchema> = ConfigService::for_plugin(&f.env, &f.ctx);
    let mut log = IncidentLog::new();

    svc.set_key("limits", r#"{"retries":7,"deep":[1,{"n":2}]}"#, &mut log)
        .expect("a complex key accepts JSON");

    let text = fs::read_to_string(&f.user_config).expect("file written");
    assert!(
        !text.contains("$serde_json::private::Number"),
        "a number was written as serde_json's private arbitrary-precision \
         marker instead of as a number — the JSON→YAML bridge regressed to \
         `serde_yaml_ng::to_value`; see quoin#442:\n{text}"
    );
    assert!(text.contains("retries: 7"), "body: {text:?}");
    assert!(text.contains("n: 2"), "nested number: {text:?}");
}
