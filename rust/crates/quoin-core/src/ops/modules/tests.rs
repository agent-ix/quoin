// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! What the four `modules` operations do, proved against an in-memory host.
//!
//! One concern per file: this one is behaviour — which host call an operation
//! makes, what it refuses before making any, and what the payload it writes
//! looks like. The exit-taxonomy mapping is proved in [`super::taxonomy`] and
//! the ceilings in [`super::wire`].

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use quoin_modules::ModulesErrorCode;

use super::support::{FakeHost, manifest_yaml, module};
use super::*;

#[test]
fn list_returns_the_registry_in_registry_order() {
    let host = FakeHost {
        modules: vec![module("zulu"), module("alpha")],
        ..FakeHost::default()
    };
    let capabilities = Capabilities::with_modules(&host);
    let response = list(&serde_json::json!({ "home": "/h" }), &capabilities).unwrap();
    assert_eq!(response.outcome.code(), 0);
    assert_eq!(response.payload["modules"][0]["name"], "zulu");
    assert_eq!(response.payload["modules"][1]["name"], "alpha");
    assert_eq!(host.calls.borrow()[0], "list(Some(\"/h\"))");
}

#[test]
fn an_absent_home_reaches_the_host_as_none_rather_than_as_a_guess() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    list(&serde_json::json!({}), &capabilities).unwrap();
    assert_eq!(host.calls.borrow()[0], "list(None)");
}

/// The payload keeps ts-plugin-kit's camelCase record shape, because
/// `~/.ix/filament/registry.json` is the SAME file the retained TypeScript
/// still reads during coexistence (NFR-024).
#[test]
fn the_record_shape_is_the_registrys_own() {
    let host = FakeHost::with_module("alpha");
    let capabilities = Capabilities::with_modules(&host);
    let response = list(&serde_json::json!({}), &capabilities).unwrap();
    let record = &response.payload["modules"][0];
    assert_eq!(record["resolvedPath"], "/cache/m");
    assert_eq!(record["targetPath"], "/home/filament/modules/m");
    assert_eq!(record["installedAt"], "2026-01-01T00:00:00Z");
    assert!(record.get("resolved_path").is_none());
    // Absent, not null — `skip_serializing_if` on the optional fields.
    assert!(record.get("sha").is_none());
    assert!(record.get("ref").is_none());
}

/// The four `parseSourceArg` spellings `src/plugins.ts` accepted, each
/// reaching the host as the source type it named.
#[test]
fn the_source_argument_spellings_are_the_typescripts_own() {
    for (arg, expected) in [
        ("path:/tmp/m", "path"),
        ("/tmp/m", "path"),
        ("github:agent-ix/spec-objects", "github"),
        ("github:agent-ix/mono//pkg@v1", "git-subdir"),
        ("package:@scope/mod@1.2.3", "npm"),
    ] {
        let host = FakeHost::default();
        let capabilities = Capabilities::with_modules(&host);
        install(&serde_json::json!({ "source": arg }), &capabilities).unwrap();
        assert_eq!(
            host.calls.borrow()[0],
            format!("install(None,{expected})"),
            "{arg}"
        );
    }
}

/// A source argument the build cannot parse is answered WITHOUT consulting
/// the host at all — the boundary refuses on its own words.
#[test]
fn an_unparsable_source_never_reaches_the_host() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    let error = install(&serde_json::json!({ "source": "github:" }), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert_eq!(error.outcome().code(), 3);
    assert!(host.calls.borrow().is_empty());
}

/// A module name that escapes its directory is refused before anything
/// holding a filesystem capability sees it.
#[test]
fn a_path_escaping_module_name_never_reaches_the_host() {
    for name in ["..", "a/b", "-flag", ""] {
        let host = FakeHost::default();
        let capabilities = Capabilities::with_modules(&host);
        let error = remove(&serde_json::json!({ "name": name }), &capabilities).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest, "{name}");
        assert!(host.calls.borrow().is_empty(), "{name}");
    }
}

#[test]
fn removing_an_absent_module_is_refused_not_internal() {
    let host = FakeHost::failing(ModulesErrorCode::ModuleNotInstalled);
    let capabilities = Capabilities::with_modules(&host);
    let error = remove(&serde_json::json!({ "name": "gone" }), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(error.context["modules_code"], "QM019_MODULE_NOT_INSTALLED");
}

/// A semantic-contract violation is exit 2 and carries the module code, so
/// the caller can tell "quoin rejected this module" from "quoin broke".
/// This is the rejection `src/plugins.ts` raised as a thrown `Error`.
#[test]
fn a_semantic_contract_violation_is_refused_and_names_its_code() {
    let host = FakeHost::failing(ModulesErrorCode::SemanticContractViolation);
    let capabilities = Capabilities::with_modules(&host);
    let error = install(&serde_json::json!({ "source": "path:/m" }), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(
        error.context["modules_code"],
        ModulesErrorCode::SemanticContractViolation.as_str()
    );
}

#[test]
fn a_registry_write_failure_is_internal_not_a_refusal() {
    let host = FakeHost::failing(ModulesErrorCode::RegistryUnwritable);
    let capabilities = Capabilities::with_modules(&host);
    let error = list(&serde_json::json!({}), &capabilities).unwrap_err();
    assert_eq!(error.code, CoreErrorCode::Io);
    assert_eq!(error.outcome().code(), 4);
}

/// The reconcile default is `lazy`. `ensureDefaultModules` ran on the
/// authoring path, so a default of `sync` would put a git fetch in front of
/// every `quoin write`.
#[test]
fn the_reconcile_default_is_lazy_and_sync_is_opt_in() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    ensure_defaults(
        &serde_json::json!({ "manifest": manifest_yaml() }),
        &capabilities,
    )
    .unwrap();
    assert!(host.calls.borrow()[0].ends_with(",Lazy)"));

    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    let response = ensure_defaults(
        &serde_json::json!({ "manifest": manifest_yaml(), "mode": "sync" }),
        &capabilities,
    )
    .unwrap();
    assert!(host.calls.borrow()[0].ends_with(",Sync)"));
    assert_eq!(response.payload["installed"][0], "alpha");
    assert_eq!(response.payload["unchanged"][0], "beta");
    assert_eq!(response.payload["updated"], serde_json::json!([]));
}

/// Reconciling re-judges what is already installed, and a module that no
/// longer satisfies its contract refuses the whole operation.
///
/// The reconcile path is the only one where "accepted at install time" and
/// "acceptable now" can differ — a module's manifest can be edited on disk
/// after it was installed, and no install-time failure stands in for that. The
/// assertion is on the call sequence as well as the refusal: without the
/// `validate_installed` call, `ensure_defaults` would report a clean reconcile
/// over a tampered tree, and every other unit test here would stay green.
#[test]
fn reconciling_revalidates_what_is_installed_and_refuses_a_tampered_module() {
    let host = FakeHost {
        tampered: Some("alpha"),
        ..FakeHost::default()
    };
    let capabilities = Capabilities::with_modules(&host);
    let error = ensure_defaults(
        &serde_json::json!({ "manifest": manifest_yaml() }),
        &capabilities,
    )
    .unwrap_err();

    assert_eq!(error.code, CoreErrorCode::Refused);
    assert_eq!(error.outcome().code(), 2);
    assert_eq!(
        error.context["modules_code"],
        ModulesErrorCode::SemanticContractViolation.as_str()
    );
    assert_eq!(error.context["op"], "modules.ensure_defaults");

    let calls = host.calls.borrow();
    assert!(
        calls[0].starts_with("ensure_defaults("),
        "reconcile ran second or not at all: {calls:?}"
    );
    assert!(
        calls[1].starts_with("validate_installed("),
        "reconcile did not re-validate the installed modules: {calls:?}"
    );
}

/// The same path with nothing tampered still re-validates, so the assertion
/// above is about the refusal and not about the call happening at all.
#[test]
fn reconciling_revalidates_even_when_everything_is_clean() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    ensure_defaults(
        &serde_json::json!({ "manifest": manifest_yaml() }),
        &capabilities,
    )
    .unwrap();
    let calls = host.calls.borrow();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(calls[1].starts_with("validate_installed("), "{calls:?}");
}

#[test]
fn a_malformed_manifest_never_reaches_the_host() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    let error = ensure_defaults(
        &serde_json::json!({ "manifest": "schemaVersion: 99\nentries: []\n" }),
        &capabilities,
    )
    .unwrap_err();
    assert_eq!(error.code, CoreErrorCode::BadRequest);
    assert!(host.calls.borrow().is_empty());
}

/// A misspelled field is refused by name on every operation, not silently
/// dropped on any of them.
#[test]
fn an_unknown_field_is_bad_request_on_every_operation() {
    let host = FakeHost::default();
    let capabilities = Capabilities::with_modules(&host);
    let cases: [(&str, serde_json::Value); 4] = [
        ("modules.list", serde_json::json!({ "hom": "/h" })),
        (
            "modules.install",
            serde_json::json!({ "source": "path:/m", "hom": "/h" }),
        ),
        (
            "modules.remove",
            serde_json::json!({ "name": "m", "hom": "/h" }),
        ),
        (
            "modules.ensure_defaults",
            serde_json::json!({ "manifest": "", "hom": "/h" }),
        ),
    ];
    for (op, request) in cases {
        let error = crate::dispatch::dispatch(op, &request, &capabilities).unwrap_err();
        assert_eq!(error.code, CoreErrorCode::BadRequest, "{op}");
        assert_eq!(error.context["op"], op);
    }
}

/// Every bound refuses at its own ceiling, before the host is consulted.
#[test]
fn each_bound_refuses_before_the_host_is_consulted() {
    let cases: [(&str, &str, &str, usize); 4] = [
        ("modules.list", "home", "home", MAX_SCALAR_BYTES),
        ("modules.install", "source", "source", MAX_SOURCE_ARG_BYTES),
        ("modules.remove", "name", "name", MAX_SCALAR_BYTES),
        (
            "modules.ensure_defaults",
            "manifest",
            "manifest",
            MAX_MANIFEST_BYTES,
        ),
    ];
    for (op, field, key, limit) in cases {
        let host = FakeHost::default();
        let capabilities = Capabilities::with_modules(&host);
        let mut request = serde_json::Map::new();
        request.insert(
            key.to_owned(),
            serde_json::Value::String("x".repeat(limit + 1)),
        );
        // Fill the other required field so the refusal is the bound and not
        // a missing key.
        if op == "modules.install" && key != "source" {
            request.insert("source".to_owned(), serde_json::json!("path:/m"));
        }
        if op == "modules.remove" && key != "name" {
            request.insert("name".to_owned(), serde_json::json!("m"));
        }
        if op == "modules.ensure_defaults" && key != "manifest" {
            request.insert("manifest".to_owned(), serde_json::json!(""));
        }
        let error =
            crate::dispatch::dispatch(op, &serde_json::Value::Object(request), &capabilities)
                .unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Refused, "{op}/{field}");
        assert_eq!(error.outcome().code(), 2, "{op}/{field}");
        assert_eq!(error.context["field"], field);
        assert_eq!(error.context["limit_bytes"], limit.to_string());
        assert!(host.calls.borrow().is_empty(), "{op}/{field}");
    }
}

/// A dispatch that reaches a module operation with no host granted is an
/// INTERNAL fault (4), not a refusal (2). The caller did nothing wrong and
/// must not be told its module was declined.
#[test]
fn a_missing_host_is_internal_not_a_refusal() {
    let none = Capabilities::none();
    for result in [
        list(&serde_json::json!({}), &none),
        install(&serde_json::json!({ "source": "path:/m" }), &none),
        remove(&serde_json::json!({ "name": "m" }), &none),
        ensure_defaults(&serde_json::json!({ "manifest": manifest_yaml() }), &none),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, CoreErrorCode::Io);
        assert_eq!(error.outcome().code(), 4);
    }
}
