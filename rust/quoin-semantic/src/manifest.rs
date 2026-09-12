// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! The `semantic` manifest block (FR-070, issue #293).
//!
//! Port of `src/semantic/manifest.ts`. Quoin reads the block at
//! `quoin module install` and rejects a manifest whose block is outside the
//! contract rather than degrading to an empty model. The block is optional: a
//! manifest without it loads exactly as before.
//!
//! This module owns the port's sharpest edge — `mapAjvError`. The TypeScript
//! reads ajv's error objects and turns four keywords into specific diagnostic
//! codes, falling through to `semantic.invalid-value` for everything else. That
//! mapping is reproduced here against [`crate::schema`]'s ajv-shaped errors; see
//! `DIVERGENCE.md` for where the two validators disagree and what each
//! disagreement costs the user.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::contract::{
    module_manifest_schema_path, semantic_core_dir, sweep_report_schema_path, SEMANTIC_CONTRACT,
};
use crate::data_schema::{resolve_data_schema, DataSchemaForm, ResolveContext, ResolvedDataSchema};
use crate::diagnostic::{DiagnosticCode, SemanticDiagnostic, Severity};
use crate::error::SemanticError;
use crate::ids::{
    ContractVersion, MappingName, ModuleName, ModuleVersion, ObjectTypeName, PackageIdentity,
    SemanticCoreVersion,
};
use crate::schema::{read_json, SchemaError, SchemaErrorParams, SchemaKeyword, SchemaValidator};

/// How a module treats a downstream consumer's additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompatibilityPosture {
    /// No additions admitted.
    Strict,
    /// Additions admitted. The schema's default.
    Additive,
    /// Additions admitted and their loss declared.
    DeclaredLossy,
}

impl CompatibilityPosture {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Strict => "strict",
            Self::Additive => "additive",
            Self::DeclaredLossy => "declared-lossy",
        }
    }

    /// Parse a wire spelling.
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "strict" => Some(Self::Strict),
            "additive" => Some(Self::Additive),
            "declared-lossy" => Some(Self::DeclaredLossy),
            _ => None,
        }
    }
}

/// Severity of legacy Properties forms (FR-074).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LegacyForms {
    /// Advisory. The schema's default.
    Warning,
    /// Blocking, and only admissible with a shipped sweep report.
    Error,
}

impl LegacyForms {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    /// Parse a wire spelling.
    #[must_use]
    pub fn from_str_opt(value: &str) -> Option<Self> {
        match value {
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// One module's validated `semantic` block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticBlock {
    /// `contract_version`.
    pub contract_version: ContractVersion,
    /// `semantic_core`.
    pub semantic_core: SemanticCoreVersion,
    /// `package`.
    pub package: PackageIdentity,
    /// `exports`, in manifest order.
    pub exports: Vec<ObjectTypeName>,
    /// `imports`, package identity to exact version.
    pub imports: BTreeMap<PackageIdentity, ModuleVersion>,
    /// `targets`, in manifest order.
    pub targets: Vec<String>,
    /// `mappings`, in manifest order.
    pub mappings: Vec<MappingName>,
    /// `compatibility_posture`, defaulted to `additive` when absent.
    pub compatibility_posture: CompatibilityPosture,
    /// `legacy_forms`, defaulted to `warning` when absent.
    pub legacy_forms: LegacyForms,
    /// `sweep_report`, when the manifest carries one as a string.
    pub sweep_report: Option<String>,
}

/// A module with a `semantic` block, plus its resolved object-type schemas.
#[derive(Debug, Clone)]
pub struct SemanticModule {
    /// The module's `name`.
    pub name: ModuleName,
    /// The module's `version`.
    pub version: ModuleVersion,
    /// The module's root directory.
    pub root: PathBuf,
    /// The validated block.
    pub block: SemanticBlock,
    /// Resolved `data_schema` per object type name.
    pub data_schemas: BTreeMap<ObjectTypeName, ResolvedDataSchema>,
}

/// Reading one manifest's `semantic` block.
#[derive(Debug, Clone)]
pub struct SemanticReadResult {
    /// The module, when the block validated. Absent when it did not, or when
    /// there is no block at all.
    pub module: Option<SemanticModule>,
    /// Everything the read found wrong.
    pub diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticReadResult {
    fn refusal(diagnostic: SemanticDiagnostic) -> Self {
        Self {
            module: None,
            diagnostics: vec![diagnostic],
        }
    }

    fn nothing() -> Self {
        Self {
            module: None,
            diagnostics: Vec::new(),
        }
    }
}

/// The compiled validators this crate needs, built once from the vendored tree.
///
/// Compilation is the expensive half and the vendored schemas never change at
/// runtime, so a caller holds one of these for the life of a command rather
/// than recompiling per manifest — the same reason the TypeScript memoises its
/// two `ValidateFunction`s in module scope.
#[derive(Debug)]
pub struct SemanticValidators {
    semantic_block: SchemaValidator,
    sweep_report: SchemaValidator,
    semantic_core_dir: PathBuf,
}

impl SemanticValidators {
    /// Compile from the vendored tree rooted at `semantic_root` — the directory
    /// holding `schemas/` and `sweep-report.schema.json`.
    ///
    /// # Errors
    ///
    /// Any of the `VendoredSchema*` conditions in [`SemanticError`].
    pub fn load(semantic_root: &Path) -> Result<Self, SemanticError> {
        let manifest_schema_path = module_manifest_schema_path(semantic_root);
        let manifest_schema = read_json(&manifest_schema_path)?;
        let block_schema = manifest_schema
            .get("properties")
            .and_then(|p| p.get("semantic"))
            .ok_or(SemanticError::VendoredSchemaIncomplete {
                path: manifest_schema_path.clone(),
                pointer: "/properties/semantic",
            })?;

        let sweep_path = sweep_report_schema_path(semantic_root);
        let sweep_schema = read_json(&sweep_path)?;

        Ok(Self {
            semantic_block: SchemaValidator::compile(&manifest_schema_path, block_schema, &[])?,
            sweep_report: SchemaValidator::compile(&sweep_path, &sweep_schema, &[])?,
            semantic_core_dir: semantic_core_dir(semantic_root),
        })
    }

    /// The compiled `semantic` block validator.
    #[must_use]
    pub fn semantic_block(&self) -> &SchemaValidator {
        &self.semantic_block
    }

    /// The compiled sweep-report validator.
    #[must_use]
    pub fn sweep_report(&self) -> &SchemaValidator {
        &self.sweep_report
    }
}

/// Turn one schema error into the diagnostic the TypeScript's `mapAjvError`
/// produces for it.
///
/// The four named keywords carry quoin-authored sentences and are reproduced
/// verbatim. The fall-through embeds the validator's own message, which is the
/// one place a diagnostic's text legitimately differs from the TypeScript's —
/// the code, severity and path are identical.
#[must_use]
pub fn map_schema_error(error: &SchemaError) -> SemanticDiagnostic {
    let at = format!("semantic{}", error.instance_path.replace('/', "."));
    match (&error.keyword, &error.params) {
        (SchemaKeyword::AdditionalProperties, SchemaErrorParams::AdditionalProperty(key)) => {
            SemanticDiagnostic::error(
                DiagnosticCode::UnknownKey,
                format!("{at}.{key}"),
                format!("unknown key inside semantic: {key}"),
            )
        }
        (SchemaKeyword::Required, SchemaErrorParams::MissingProperty(key)) => {
            SemanticDiagnostic::error(
                DiagnosticCode::MissingKey,
                format!("{at}.{key}"),
                format!("semantic block requires {key}"),
            )
        }
        (SchemaKeyword::Enum, SchemaErrorParams::AllowedValues(allowed)) => {
            let code = if is_targets_locus(&at) {
                DiagnosticCode::UnknownTarget
            } else {
                DiagnosticCode::InvalidValue
            };
            SemanticDiagnostic::error(
                code,
                at.clone(),
                format!(
                    "{at} is {}, not one of {}",
                    json_text(&error.instance),
                    json_text(&Value::Array(allowed.clone()))
                ),
            )
        }
        (SchemaKeyword::Pattern, SchemaErrorParams::Pattern(pattern)) => {
            let code = if at.ends_with("package") {
                DiagnosticCode::InvalidPackage
            } else {
                DiagnosticCode::InvalidValue
            };
            SemanticDiagnostic::error(code, at.clone(), format!("{at} does not match {pattern}"))
        }
        _ => SemanticDiagnostic::error(
            DiagnosticCode::InvalidValue,
            at.clone(),
            format!("{at}: {}", error.message),
        ),
    }
}

/// `at.endsWith("targets") || /targets\.\d+$/.test(at)` from the TypeScript.
fn is_targets_locus(at: &str) -> bool {
    if at.ends_with("targets") {
        return true;
    }
    let Some((head, index)) = at.rsplit_once('.') else {
        return false;
    };
    !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()) && head.ends_with("targets")
}

/// `JSON.stringify(value)`, which is what the TypeScript message interpolates.
fn json_text(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".to_owned())
}

/// Read and validate the `semantic` block of one parsed manifest.
///
/// `manifest` is the manifest document as JSON — the caller has already parsed
/// the YAML, exactly as the TypeScript's caller has.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn read_semantic_block(
    manifest: &Value,
    module_root: &Path,
    validators: &SemanticValidators,
) -> SemanticReadResult {
    let Some(manifest_map) = manifest.as_object() else {
        return SemanticReadResult::nothing();
    };
    let Some(raw) = manifest_map.get("semantic") else {
        return SemanticReadResult::nothing();
    };
    let Some(raw_map) = raw.as_object() else {
        return SemanticReadResult::refusal(SemanticDiagnostic::error(
            DiagnosticCode::InvalidValue,
            "semantic",
            "semantic must be an object",
        ));
    };

    // The contract version gates everything else: an unknown version is refused
    // before any other key is read (FR-070).
    let declared_contract_version = raw_map.get("contract_version");
    if declared_contract_version.and_then(Value::as_str) != Some(SEMANTIC_CONTRACT.contract_version)
    {
        let shown = declared_contract_version.map_or_else(
            || "undefined".to_owned(),
            |v| v.as_str().map_or_else(|| stringify(v), str::to_owned),
        );
        return SemanticReadResult::refusal(SemanticDiagnostic::error(
            DiagnosticCode::UnsupportedContractVersion,
            "semantic.contract_version",
            format!(
                "semantic.contract_version {shown} is not {}",
                SEMANTIC_CONTRACT.contract_version
            ),
        ));
    }

    let mut diagnostics: Vec<SemanticDiagnostic> = Vec::new();

    if !validators.semantic_block.is_valid(raw) {
        for error in validators.semantic_block.errors(raw) {
            diagnostics.push(map_schema_error(&error));
        }
        return SemanticReadResult {
            module: None,
            diagnostics,
        };
    }

    let object_types: Vec<&serde_json::Map<String, Value>> = manifest_map
        .get("object_types")
        .and_then(Value::as_array)
        .map(|entries| entries.iter().filter_map(Value::as_object).collect())
        .unwrap_or_default();
    let object_names: std::collections::BTreeSet<String> = object_types
        .iter()
        .map(|entry| entry.get("name").map_or_else(String::new, stringify))
        .collect();

    let exports: Vec<ObjectTypeName> = string_array(raw_map.get("exports"))
        .into_iter()
        .map(ObjectTypeName::new)
        .collect();

    for name in &exports {
        if !object_names.contains(name.as_str()) {
            diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::UnknownExport,
                format!("semantic.exports.{name}"),
                format!("semantic.exports names {name}, which object_types does not declare"),
            ));
        }
    }

    let semantic_core = SemanticCoreVersion::new(
        raw_map
            .get("semantic_core")
            .map_or_else(String::new, stringify),
    );
    if !SEMANTIC_CONTRACT.ships_semantic_core(&semantic_core) {
        diagnostics.push(SemanticDiagnostic::error(
            DiagnosticCode::UnknownSemanticCore,
            "semantic.semantic_core",
            format!(
                "semantic_core {semantic_core} is not a version this quoin ships ({})",
                SEMANTIC_CONTRACT.semantic_core_versions.join(", ")
            ),
        ));
    }

    let module_version = ModuleVersion::new(
        manifest_map
            .get("version")
            .map_or_else(String::new, stringify),
    );

    let block = SemanticBlock {
        contract_version: ContractVersion::new(
            raw_map
                .get("contract_version")
                .map_or_else(String::new, stringify),
        ),
        semantic_core: semantic_core.clone(),
        package: PackageIdentity::new(raw_map.get("package").map_or_else(String::new, stringify)),
        exports,
        imports: raw_map
            .get("imports")
            .and_then(Value::as_object)
            .map(|map| {
                map.iter()
                    .map(|(k, v)| {
                        (
                            PackageIdentity::new(k.clone()),
                            ModuleVersion::new(stringify(v)),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        targets: string_array(raw_map.get("targets")),
        mappings: string_array(raw_map.get("mappings"))
            .into_iter()
            .map(MappingName::new)
            .collect(),
        compatibility_posture: raw_map
            .get("compatibility_posture")
            .and_then(Value::as_str)
            .and_then(CompatibilityPosture::from_str_opt)
            .unwrap_or(CompatibilityPosture::Additive),
        legacy_forms: raw_map
            .get("legacy_forms")
            .and_then(Value::as_str)
            .and_then(LegacyForms::from_str_opt)
            .unwrap_or(LegacyForms::Warning),
        sweep_report: raw_map
            .get("sweep_report")
            .and_then(Value::as_str)
            .map(str::to_owned),
    };

    if block.legacy_forms == LegacyForms::Error {
        if let Some(problem) =
            sweep_report_problem(&block, module_root, module_version.as_str(), validators)
        {
            diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::SweepReportRequired,
                "semantic.legacy_forms",
                problem,
            ));
        }
    }

    let mut data_schemas: BTreeMap<ObjectTypeName, ResolvedDataSchema> = BTreeMap::new();
    for entry in &object_types {
        let name = ObjectTypeName::new(entry.get("name").map_or_else(String::new, stringify));
        let Some(value) = entry.get("data_schema") else {
            continue;
        };
        let ctx = ResolveContext {
            module_root: module_root.to_path_buf(),
            package_identity: block.package.clone(),
            module_version: module_version.as_str().to_owned(),
            semantic_core: semantic_core.clone(),
            object_type: name.clone(),
            semantic_core_dir: validators.semantic_core_dir.clone(),
        };
        let resolved = resolve_data_schema(value, &ctx, true);
        diagnostics.extend(resolved.diagnostics.iter().cloned());
        data_schemas.insert(name, resolved);
    }

    for name in &block.exports {
        let is_reference = data_schemas
            .get(name)
            .is_some_and(|resolved| resolved.kind == DataSchemaForm::Reference);
        if !is_reference {
            diagnostics.push(SemanticDiagnostic::error(
                DiagnosticCode::ExportWithoutSchema,
                format!("semantic.exports.{name}"),
                format!(
                    "semantic.exports names {name}, whose data_schema is not a \
                     {{ schema, digest }} reference; nothing can be pinned for it"
                ),
            ));
        }
    }

    SemanticReadResult {
        module: Some(SemanticModule {
            name: ModuleName::new(manifest_map.get("name").map_or_else(String::new, stringify)),
            version: module_version,
            root: module_root.to_path_buf(),
            block,
            data_schemas,
        }),
        diagnostics,
    }
}

/// FR-074: `legacy_forms: error` needs a shipped sweep report for this package
/// and version. Returns the reason it does not, or `None` when it does.
fn sweep_report_problem(
    block: &SemanticBlock,
    module_root: &Path,
    version: &str,
    validators: &SemanticValidators,
) -> Option<String> {
    let Some(sweep_report) = block.sweep_report.as_deref() else {
        return Some("legacy_forms: error requires semantic.sweep_report".to_owned());
    };
    if sweep_report
        .split(['\\', '/'])
        .any(|segment| segment == "..")
    {
        return Some("sweep_report escapes the module root".to_owned());
    }
    let file = module_root.join(sweep_report);
    if !file.exists() {
        return Some(format!("sweep_report {sweep_report} is not shipped"));
    }
    let Ok(bytes) = std::fs::read(&file) else {
        return Some(format!("sweep_report {sweep_report} is not JSON"));
    };
    let Ok(report) = serde_json::from_slice::<Value>(&bytes) else {
        return Some(format!("sweep_report {sweep_report} is not JSON"));
    };
    let Some(map) = report.as_object() else {
        return Some(format!("sweep_report {sweep_report} is not an object"));
    };
    let declared_package = map.get("package").map_or_else(String::new, stringify);
    if declared_package != block.package.as_str() {
        return Some(format!(
            "sweep_report is for package {declared_package}, manifest is {}",
            block.package
        ));
    }
    let declared_version = map.get("version").map_or_else(String::new, stringify);
    if declared_version != version {
        return Some(format!(
            "sweep_report is for version {declared_version}, manifest is {version}"
        ));
    }
    if validators.sweep_report.is_valid(&report) {
        return None;
    }
    // The TypeScript prints ajv's *first* error. Order is not contractual across
    // the two validators, so the Rust prints the first in the deterministic
    // `(instance location, keyword)` order `SchemaValidator::errors` guarantees.
    let errors = validators.sweep_report.errors(&report);
    let first = errors.first();
    let location = first.map_or("/", |e| {
        if e.instance_path.is_empty() {
            "/"
        } else {
            e.instance_path.as_str()
        }
    });
    let detail = first.map_or("", |e| e.message.as_str());
    Some(
        format!(
            "sweep_report {sweep_report} does not validate against the sweep-report schema: \
             {location} {detail}"
        )
        .trim_end()
        .to_owned(),
    )
}

/// Read a module root's `manifest.yaml` and its semantic block.
///
/// # Errors
///
/// [`SemanticError::ManifestUnreadable`], [`SemanticError::ManifestNotYaml`] or
/// [`SemanticError::ManifestNotAMapping`].
pub fn read_module_semantic(
    module_root: &Path,
    validators: &SemanticValidators,
) -> Result<SemanticReadResult, SemanticError> {
    let manifest_path = module_root.join("manifest.yaml");
    let manifest = read_manifest_yaml(&manifest_path)?;
    Ok(read_semantic_block(&manifest, module_root, validators))
}

/// Parse a `manifest.yaml` into JSON.
///
/// # Errors
///
/// [`SemanticError::ManifestUnreadable`], [`SemanticError::ManifestNotYaml`] or
/// [`SemanticError::ManifestNotAMapping`].
pub fn read_manifest_yaml(manifest_path: &Path) -> Result<Value, SemanticError> {
    let text = std::fs::read_to_string(manifest_path).map_err(|source| {
        SemanticError::ManifestUnreadable {
            path: manifest_path.to_path_buf(),
            source,
        }
    })?;
    let manifest: Value =
        serde_norway::from_str(&text).map_err(|source| SemanticError::ManifestNotYaml {
            path: manifest_path.to_path_buf(),
            source,
        })?;
    if manifest.is_object() {
        Ok(manifest)
    } else {
        Err(SemanticError::ManifestNotAMapping {
            path: manifest_path.to_path_buf(),
        })
    }
}

/// Refuse a module whose semantic package another installed module already
/// provides (FR-070).
#[must_use]
pub fn duplicate_package_diagnostic(
    candidate: &SemanticModule,
    installed: &[SemanticModule],
) -> Option<SemanticDiagnostic> {
    let other = installed.iter().find(|module| {
        module.block.package == candidate.block.package && module.root != candidate.root
    })?;
    Some(SemanticDiagnostic::error(
        DiagnosticCode::DuplicatePackage,
        "semantic.package",
        format!(
            "semantic.package {} is already declared by module {} at {}; {} at {} is rejected \
             (sorted root order)",
            candidate.block.package,
            other.name,
            other.root.display(),
            candidate.name,
            candidate.root.display()
        ),
    ))
}

/// True when any diagnostic is error severity.
#[must_use]
pub fn has_errors(diagnostics: &[SemanticDiagnostic]) -> bool {
    diagnostics.iter().any(|d| d.severity == Severity::Error)
}

/// One line per diagnostic, in the `formatDiagnostics` shape.
#[must_use]
pub fn format_diagnostics(diagnostics: &[SemanticDiagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// `String(value)` for the scalar shapes a manifest carries.
fn stringify(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

/// A JSON array's string members; `[]` for anything else.
///
/// Non-string members are stringified, matching the TypeScript's
/// `raw.targets as string[]` — which does not filter, it only lies about the
/// type. The block has already validated by the time this runs, so the cast is
/// sound there; here it is explicit.
fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().map(stringify).collect())
        .unwrap_or_default()
}

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Trace: FR-070
    #[test]
    fn tc_378_060_targets_locus_detection_matches_the_regex() {
        assert!(is_targets_locus("semantic.targets"));
        assert!(is_targets_locus("semantic.targets.0"));
        assert!(is_targets_locus("semantic.targets.12"));
        assert!(!is_targets_locus("semantic.targets.0.x"));
        assert!(!is_targets_locus("semantic.mappings.0"));
        assert!(!is_targets_locus("semantic.targetsx"));
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_061_map_schema_error_names_each_unknown_key_separately() {
        let diagnostic = map_schema_error(&SchemaError {
            instance_path: String::new(),
            keyword: SchemaKeyword::AdditionalProperties,
            params: SchemaErrorParams::AdditionalProperty("nope".to_owned()),
            instance: json!({}),
            message: "irrelevant".to_owned(),
        });
        assert_eq!(diagnostic.code, DiagnosticCode::UnknownKey);
        assert_eq!(diagnostic.path, "semantic.nope");
        assert_eq!(diagnostic.message, "unknown key inside semantic: nope");
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_062_map_schema_error_distinguishes_target_from_value() {
        let target = map_schema_error(&SchemaError {
            instance_path: "/targets/0".to_owned(),
            keyword: SchemaKeyword::Enum,
            params: SchemaErrorParams::AllowedValues(vec![json!("rust")]),
            instance: json!("cobol"),
            message: String::new(),
        });
        assert_eq!(target.code, DiagnosticCode::UnknownTarget);
        assert_eq!(target.path, "semantic.targets.0");
        assert_eq!(
            target.message,
            r#"semantic.targets.0 is "cobol", not one of ["rust"]"#
        );

        let posture = map_schema_error(&SchemaError {
            instance_path: "/compatibility_posture".to_owned(),
            keyword: SchemaKeyword::Enum,
            params: SchemaErrorParams::AllowedValues(vec![json!("strict")]),
            instance: json!("lossy"),
            message: String::new(),
        });
        assert_eq!(posture.code, DiagnosticCode::InvalidValue);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_063_map_schema_error_distinguishes_package_from_value() {
        let package = map_schema_error(&SchemaError {
            instance_path: "/package".to_owned(),
            keyword: SchemaKeyword::Pattern,
            params: SchemaErrorParams::Pattern("^a$".to_owned()),
            instance: json!("X"),
            message: String::new(),
        });
        assert_eq!(package.code, DiagnosticCode::InvalidPackage);
        assert_eq!(package.message, "semantic.package does not match ^a$");

        let other = map_schema_error(&SchemaError {
            instance_path: "/semantic_core".to_owned(),
            keyword: SchemaKeyword::Pattern,
            params: SchemaErrorParams::Pattern("^a$".to_owned()),
            instance: json!("X"),
            message: String::new(),
        });
        assert_eq!(other.code, DiagnosticCode::InvalidValue);
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_064_map_schema_error_falls_through_to_invalid_value() {
        let diagnostic = map_schema_error(&SchemaError {
            instance_path: "/exports/0".to_owned(),
            keyword: SchemaKeyword::MinLength,
            params: SchemaErrorParams::None,
            instance: json!(""),
            message: "is shorter than 1 character".to_owned(),
        });
        assert_eq!(diagnostic.code, DiagnosticCode::InvalidValue);
        assert_eq!(diagnostic.path, "semantic.exports.0");
    }

    /// Trace: FR-070
    #[test]
    fn tc_378_065_has_errors_ignores_warnings() {
        let warning = SemanticDiagnostic::warning(DiagnosticCode::InlineDataSchema, "x", "y");
        let error = SemanticDiagnostic::error(DiagnosticCode::UnknownKey, "x", "y");
        assert!(!has_errors(std::slice::from_ref(&warning)));
        assert!(has_errors(&[warning, error]));
    }
}
