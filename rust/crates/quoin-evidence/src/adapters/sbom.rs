// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! `CycloneDX` and SPDX inventories → run entries, one per component (FR-041).
//!
//! **What an SBOM leaf in an assurance case contains, and why that settles the
//! record shape.** agent-ix/quoin#116 deferred this until FR-040 existed,
//! because the question was whether a supply-chain obligation is discharged by
//! an SBOM's *presence* or by its *contents*. FR-040 answered it: an argument
//! evidence leaf renders one line per obligation and does not render component
//! lists. So the claim an SBOM supports is **"a complete inventory was produced
//! at this commit"**, which is a run record.
//!
//! **One entry per component, deliberately.** The alternative — a single entry
//! with the count in `score` — makes an empty SBOM indistinguishable from a
//! healthy one without a new check. As entries, an inventory listing nothing
//! produces no entries, and `vacuous-evidence` reports it with no new machinery.

use serde_json::Value;

use super::AdapterResult;
use crate::error::EvidenceError;
use crate::types::{Outcome, RunEntry};

/// Read a member as a string, the way `typeof x === "string"` does: a number or
/// a `null` is not a string and does not become one.
fn string_member<'a>(object: &'a Value, name: &str) -> Option<&'a str> {
    object.get(name).and_then(Value::as_str)
}

/// Parse a `CycloneDX` or SPDX JSON inventory.
///
/// # Errors
///
/// Refuses input that is not JSON, input that is not a JSON object, and a
/// document that is neither a `CycloneDX` nor an SPDX inventory.
pub fn parse_sbom(raw: &str) -> Result<AdapterResult, EvidenceError> {
    let document: Value = serde_json::from_str(raw).map_err(|error| {
        EvidenceError::adapter(
            "sbom",
            format!(
                "not JSON: {error}. CycloneDX and SPDX both have JSON serialisations; the tag-value SPDX form is not read here."
            ),
        )
    })?;
    // `typeof document !== "object"` in the retained source, where `typeof []`
    // IS `"object"` and `typeof null` is too — so an array reaches the format
    // discrimination below and only a scalar or `null` is refused here.
    if !(document.is_object() || document.is_array()) {
        return Err(EvidenceError::adapter("sbom", "expected a JSON object"));
    }

    // No `tool` here: run-shaped adapters do not report one, and the record
    // takes it from `--tool` at record time.
    let entries = match cyclone_dx_entries(&document) {
        Some(entries) => entries,
        None => match spdx_entries(&document) {
            Some(entries) => entries,
            None => return Err(unrecognised(&document)),
        },
    };
    Ok(AdapterResult::from_entries(entries))
}

/// `CycloneDX`: `bomFormat: "CycloneDX"` with a `components` array.
fn cyclone_dx_entries(root: &Value) -> Option<Vec<RunEntry>> {
    // `String(root.bomFormat ?? "")` in the retained source: a non-string
    // `bomFormat` is stringified and will not equal `"CycloneDX"`, so only the
    // exact string selects this branch.
    if string_member(root, "bomFormat") != Some("CycloneDX") {
        return None;
    }
    let components = root.get("components").and_then(Value::as_array);
    Some(
        components
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|component| {
                // `purl` is the stable identity when present; `name@version`
                // otherwise. Neither is invented: a component with no name is a
                // malformed entry and is dropped rather than given one, because
                // a fabricated symbol would bind to nothing and inflate the
                // inventory count that proves the SBOM is not vacuous.
                let purl = string_member(component, "purl");
                let name = string_member(component, "name");
                let version = string_member(component, "version");
                symbol(purl, name, version)
            })
            .map(|symbol| RunEntry::new(symbol, Outcome::Pass))
            .collect(),
    )
}

/// SPDX JSON: `spdxVersion` with a `packages` array.
fn spdx_entries(root: &Value) -> Option<Vec<RunEntry>> {
    string_member(root, "spdxVersion")?;
    let packages = root.get("packages").and_then(Value::as_array);
    Some(
        packages
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|package| {
                let locator = package
                    .get("externalRefs")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .find(|reference| string_member(reference, "referenceType") == Some("purl"))
                    .and_then(|reference| string_member(reference, "referenceLocator"));
                let name = string_member(package, "name");
                let version = string_member(package, "versionInfo");
                symbol(locator, name, version)
            })
            .map(|symbol| RunEntry::new(symbol, Outcome::Pass))
            .collect(),
    )
}

/// `locator ?? (version ? name@version : name)`, and `None` when neither a
/// locator nor a name is present.
///
/// The empty string is falsy in JavaScript, so `version: ""` takes the
/// name-only branch and `purl: ""` falls through to the name. Both are
/// reproduced here rather than treated as present.
fn symbol(locator: Option<&str>, name: Option<&str>, version: Option<&str>) -> Option<String> {
    let locator = locator.filter(|value| !value.is_empty());
    let name = name.filter(|value| !value.is_empty());
    match (locator, name) {
        (Some(locator), _) => Some(locator.to_owned()),
        (None, Some(name)) => Some(match version.filter(|value| !value.is_empty()) {
            Some(version) => format!("{name}@{version}"),
            None => name.to_owned(),
        }),
        (None, None) => None,
    }
}

/// Neither format recognised.
///
/// # Divergence from the retained TypeScript, in the refusal text only
///
/// `Object.keys(root).slice(0, 6)` lists the document's own member order;
/// `serde_json::Map` is a `BTreeMap` (the `preserve_order` feature is
/// deliberately off workspace-wide, because enabling it silently breaks the
/// canonical-JSON comparison the native fixture suite performs), so this lists the
/// first six member names in byte order instead. The set of names is the same
/// for a document with six or fewer members; for a larger one the six named
/// may differ. Nothing but this diagnostic reads them, and no record byte
/// changes. Recorded in quoin#456.
///
/// Rejected rather than returning zero entries. Zero entries means "the SBOM
/// listed nothing", which `vacuous-evidence` reports as a real finding about
/// the consumer's build — and a file this adapter simply could not read must
/// not masquerade as that.
fn unrecognised(root: &Value) -> EvidenceError {
    // `Object.keys(root).slice(0, 6)`: an object's own keys, or an array's
    // indices rendered as decimal strings.
    let keys: Vec<String> = match root {
        Value::Object(object) => object.keys().take(6).cloned().collect(),
        Value::Array(items) => (0..items.len().min(6))
            .map(|index| index.to_string())
            .collect(),
        _ => Vec::new(),
    };
    let listed = if keys.is_empty() {
        "(none)".to_owned()
    } else {
        keys.join(", ")
    };
    EvidenceError::adapter(
        "sbom",
        format!(
            "neither a CycloneDX (`bomFormat`) nor an SPDX (`spdxVersion`) document — top-level keys: {listed}"
        ),
    )
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]
mod tests {
    use super::parse_sbom;

    /// One entry per component, keyed on the purl where the document carries
    /// one — and a component with no identity at all is dropped rather than
    /// given a fabricated symbol that binds to nothing.
    ///
    /// Trace: FR-041-AC-1, FR-041-AC-5, FR-041-CON-4
    /// Provenance: quoin#458
    #[test]
    fn tc_458_230_cyclonedx_prefers_purl_then_name_at_version_then_name() {
        let result = parse_sbom(
            r#"{"bomFormat":"CycloneDX","components":[
                {"purl":"pkg:npm/a@1","name":"a","version":"1"},
                {"name":"b","version":"2"},
                {"name":"c"},
                {"version":"4"}
            ]}"#,
        )
        .unwrap();
        let symbols: Vec<&str> = result.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(symbols, ["pkg:npm/a@1", "b@2", "c"]);
        // The catalog and the suite registry carry the evidence-kind
        // vocabulary; an adapter that minted a fourth copy would be declaring
        // what only the consumer can say.
        assert!(result.evidence_kind.is_none());
    }

    /// SPDX puts the purl in `externalRefs`, not in a top-level field: reading
    /// `name` alone would give the same component a different identity
    /// depending on which format produced it.
    ///
    /// Trace: FR-041-AC-2
    /// Provenance: quoin#458
    #[test]
    fn tc_458_231_spdx_reads_the_purl_external_ref() {
        let result = parse_sbom(
            r#"{"spdxVersion":"SPDX-2.3","packages":[
                {"name":"a","versionInfo":"1","externalRefs":[
                    {"referenceType":"cpe23Type","referenceLocator":"cpe:x"},
                    {"referenceType":"purl","referenceLocator":"pkg:cargo/a@1"}]},
                {"name":"b","versionInfo":"2"}
            ]}"#,
        )
        .unwrap();
        let symbols: Vec<&str> = result.entries.iter().map(|e| e.symbol.as_str()).collect();
        assert_eq!(symbols, ["pkg:cargo/a@1", "b@2"]);
    }

    /// Trace: FR-041-AC-3
    /// Provenance: quoin#458
    #[test]
    fn tc_458_232_an_empty_inventory_is_entries_zero_and_not_a_refusal() {
        // The distinction the record type exists to make: a tool that ran and
        // listed nothing is a finding about the build, not an unreadable file.
        let result = parse_sbom(r#"{"bomFormat":"CycloneDX","components":[]}"#).unwrap();
        assert!(result.entries.is_empty());
    }

    /// Zero entries is a real finding about the consumer's build; a file the
    /// adapter simply could not read must not masquerade as one.
    ///
    /// Trace: FR-041-AC-4
    /// Provenance: quoin#458
    #[test]
    fn tc_458_233_an_unrecognised_document_names_its_first_six_keys() {
        let error = parse_sbom(r#"{"a":1,"b":2,"c":3,"d":4,"e":5,"f":6,"g":7}"#).unwrap_err();
        assert_eq!(
            error.to_string(),
            "sbom: neither a CycloneDX (`bomFormat`) nor an SPDX (`spdxVersion`) document — top-level keys: a, b, c, d, e, f"
        );
        let empty = parse_sbom("{}").unwrap_err();
        assert!(empty.to_string().ends_with("top-level keys: (none)"));
        assert!(
            parse_sbom("not json at all")
                .unwrap_err()
                .to_string()
                .contains("not JSON")
        );
    }
}
