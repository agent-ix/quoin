// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Rust adapter for the native `quoin update` delivery path.

use std::collections::BTreeMap;
use std::path::Path;

use clap::{Arg, ArgAction, ArgMatches, Command};
use ix_cli_kit::exit::Outcome;
use ix_trace_rs::implements;
use quoin_core::protocol::{Diagnostic, Response};
use quoin_delivery::{DEFAULT_MANIFEST_URL, DeliveryError, HttpTransport, Transport, UpdateResult};
use semver::Version;

pub(crate) fn command() -> Command {
    Command::new("update")
        .about("Check for and install the latest stable quoin release")
        .arg(Arg::new("check").long("check").action(ArgAction::SetTrue))
        .arg(
            Arg::new("registry")
                .long("registry")
                .value_name("MANIFEST_URL")
                .help("Override the release-manifest endpoint for a local development snapshot"),
        )
}

pub(crate) fn run(arguments: &ArgMatches) -> Result<Response, String> {
    let executable = std::env::current_exe()
        .map_err(|error| format!("cannot locate quoin executable: {error}"))?;
    run_with(arguments, &executable, &HttpTransport)
}

/// True when `executable` lives inside an npm package tree, where npm owns the
/// installed bytes and its recorded version.
fn npm_managed(executable: &Path) -> bool {
    executable
        .components()
        .any(|part| part.as_os_str() == "node_modules")
}

fn npm_refusal() -> Response {
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: "update.npm_managed".to_owned(),
            message: "this quoin was installed by npm; update it with \
                      `npm install -g @agent-ix/quoin@latest`"
                .to_owned(),
            context: BTreeMap::new(),
        }],
        outcome: Outcome::Refused,
    }
}

#[implements("FR-022-AC-3")]
fn run_with(
    arguments: &ArgMatches,
    executable: &Path,
    transport: &dyn Transport,
) -> Result<Response, String> {
    if npm_managed(executable) {
        return Ok(npm_refusal());
    }
    let custom_manifest = arguments.contains_id("registry");
    let manifest = arguments
        .get_one::<String>("registry")
        .map_or(DEFAULT_MANIFEST_URL, String::as_str);
    let current = Version::parse(env!("QUOIN_VERSION"))
        .or_else(|_| Version::parse(env!("CARGO_PKG_VERSION")))
        .map_err(|error| format!("native build version is not SemVer: {error}"))?;
    let response = quoin_delivery::update(
        transport,
        manifest,
        custom_manifest,
        &current,
        executable,
        arguments.get_flag("check"),
    );
    Ok(match response {
        Ok(result) => success(result, &current),
        Err(error) => failure(&error),
    })
}

fn success(result: UpdateResult, current: &Version) -> Response {
    match result {
        UpdateResult::Current { version } => Response::ok(serde_json::json!({
            "rendered": format!("quoin update: already at {current} (latest {version})")
        })),
        UpdateResult::Available { version } => Response {
            payload: serde_json::json!({
                "rendered": format!("quoin update: current {current}, latest {version}")
            }),
            diagnostics: Vec::new(),
            outcome: Outcome::Partial,
        },
        UpdateResult::Replaced { version } => Response::ok(serde_json::json!({
            "rendered": format!("quoin update: updated {current} → {version}")
        })),
        UpdateResult::Staged { version, path } => Response::ok(serde_json::json!({
            "rendered": format!("quoin update: staged {current} → {version} at {}; replace the executable after quoin exits", path.display())
        })),
    }
}

fn failure(error: &DeliveryError) -> Response {
    let outcome = match error.outcome() {
        quoin_delivery::quoin_outcome::OutcomeClass::Refused => Outcome::Refused,
        quoin_delivery::quoin_outcome::OutcomeClass::Invalid => Outcome::Invalid,
        quoin_delivery::quoin_outcome::OutcomeClass::Internal => Outcome::Internal,
    };
    Response {
        payload: serde_json::Value::Null,
        diagnostics: vec![Diagnostic {
            code: format!("update.{:?}", error.outcome()).to_lowercase(),
            message: error.to_string(),
            context: BTreeMap::new(),
        }],
        outcome,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test fixtures may panic")]
mod tests {
    use ix_trace_rs::trace;
    use std::cell::RefCell;
    use std::path::Path;

    use quoin_core::protocol::Response;
    use quoin_delivery::{DEFAULT_MANIFEST_URL, DeliveryError, Transport};
    use url::Url;

    use super::{command, run_with};

    /// Fails every fetch, so a run that reaches the transport is observable.
    struct Unreachable;

    impl Transport for Unreachable {
        fn get(&self, _url: &Url, _limit: usize) -> Result<Vec<u8>, DeliveryError> {
            Err(DeliveryError::Manifest("transport reached".to_owned()))
        }
    }

    struct RecordingTransport {
        requested: RefCell<Vec<String>>,
    }

    impl Transport for RecordingTransport {
        fn get(&self, url: &Url, _limit: usize) -> Result<Vec<u8>, DeliveryError> {
            self.requested.borrow_mut().push(url.as_str().to_owned());
            Err(DeliveryError::Manifest(
                "stop after endpoint capture".to_owned(),
            ))
        }
    }

    fn run_update(args: &[&str], executable: &str) -> Response {
        let matches = command()
            .try_get_matches_from(args)
            .expect("grammar parses");
        run_with(&matches, Path::new(executable), &Unreachable).expect("update runs")
    }

    #[trace("FR-022-AC-4")]
    #[test]
    fn tc_1013_update_refuses_inside_node_modules_without_touching_the_network() {
        let response = run_update(
            &["update"],
            "/usr/lib/node_modules/@agent-ix/quoin-linux-x64/bin/quoin",
        );
        assert_eq!(response.outcome, ix_cli_kit::exit::Outcome::Refused);
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_eq!(diagnostic.code, "update.npm_managed");
        assert!(
            diagnostic
                .message
                .contains("npm install -g @agent-ix/quoin@latest")
        );
    }

    #[trace("FR-022-AC-4")]
    #[test]
    fn tc_1013_update_outside_node_modules_reaches_the_github_release_path() {
        let response = run_update(&["update"], "/usr/local/bin/quoin");
        let diagnostic = response.diagnostics.first().expect("diagnostic");
        assert_ne!(diagnostic.code, "update.npm_managed");
        assert!(diagnostic.message.contains("transport reached"));
    }

    #[trace("FR-022", "FR-001-AC-3")]
    #[test]
    fn tc_373_751_update_preserves_check_and_registry_grammar() {
        let matches = command()
            .try_get_matches_from([
                "update",
                "--check",
                "--registry",
                "http://localhost/manifest.json",
            ])
            .expect("grammar parses");
        assert!(matches.get_flag("check"));
        assert_eq!(
            matches.get_one::<String>("registry").map(String::as_str),
            Some("http://localhost/manifest.json")
        );
    }

    #[trace("FR-022-AC-3")]
    #[test]
    fn tc_373_752_update_without_registry_selects_stable_github_manifest() {
        let matches = command()
            .try_get_matches_from(["update"])
            .expect("grammar parses without an override");
        let transport = RecordingTransport {
            requested: RefCell::new(Vec::new()),
        };
        let _response = run_with(&matches, Path::new("/usr/local/bin/quoin"), &transport)
            .expect("update reaches its injected transport");
        assert_eq!(
            *transport.requested.borrow(),
            [DEFAULT_MANIFEST_URL],
            "omitting --registry requests the stable manifest endpoint"
        );
        assert_eq!(
            "https://github.com/agent-ix/quoin/releases/latest/download/quoin-update-manifest.json",
            DEFAULT_MANIFEST_URL
        );
    }

    #[trace("FR-022-AC-3")]
    #[test]
    fn tc_373_753_update_with_registry_uses_explicit_manifest_endpoint() {
        let registry = "http://localhost/manifest.json";
        let matches = command()
            .try_get_matches_from(["update", "--registry", registry])
            .expect("grammar parses the explicit registry");
        let transport = RecordingTransport {
            requested: RefCell::new(Vec::new()),
        };
        let _response = run_with(&matches, Path::new("/usr/local/bin/quoin"), &transport)
            .expect("update reaches its injected transport");

        assert_eq!(
            *transport.requested.borrow(),
            [registry],
            "--registry must select the endpoint used by the transport"
        );
    }
}
