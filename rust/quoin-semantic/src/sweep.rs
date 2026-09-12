// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2026 Agent-IX

//! Legacy Properties-form detection and the advisory sweep (FR-074, issue #293).
//!
//! Port of `src/semantic/sweep.ts`. Quire emits the per-artifact
//! `semantic.legacy-properties-form` warning at validation time (quire-rs#388);
//! this is Quoin's side — the same classifier, run over a corpus root by
//! `quoin semantic sweep`, producing the report that `semantic.legacy_forms:
//! error` must cite before a module may promote the warning.
//!
//! The classifier reads Markdown as text: it never rewrites an artifact.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::ids::PackageIdentity;

/// The four shapes a `## Properties` section can take, plus its absence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PropertiesForm {
    /// The authored form: the four typed columns.
    TypedTable,
    /// A table whose header is not the typed four. Legacy.
    FreeColumnTable,
    /// A bullet list. Legacy.
    BulletList,
    /// A `sysml` fenced block.
    SysmlFence,
    /// No `## Properties` section, or one with no block in it.
    None,
}

impl PropertiesForm {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypedTable => "typed-table",
            Self::FreeColumnTable => "free-column-table",
            Self::BulletList => "bullet-list",
            Self::SysmlFence => "sysml-fence",
            Self::None => "none",
        }
    }

    /// Every form, in report order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            Self::TypedTable,
            Self::FreeColumnTable,
            Self::BulletList,
            Self::SysmlFence,
            Self::None,
        ]
    }

    /// True for the two forms FR-074 asks authors to migrate away from.
    #[must_use]
    pub fn is_legacy(self) -> bool {
        matches!(self, Self::FreeColumnTable | Self::BulletList)
    }
}

/// The four columns the authored form declares.
pub const TYPED_HEADER: [&str; 4] = ["Field", "Type", "Multiplicity", "Constraints"];

/// The advisory diagnostic a legacy form earns (FR-074).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LegacyFormDiagnostic {
    /// Always `semantic.legacy-properties-form`.
    pub code: &'static str,
    /// Always `warning`.
    pub severity: &'static str,
    /// Which legacy form was found.
    pub form: PropertiesForm,
    /// 1-based line of the block.
    pub line: usize,
    /// Always `typed-table`.
    pub migration: &'static str,
}

/// One classified artifact.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FormFinding {
    /// Repo-relative artifact path, prefixed with its repository.
    pub path: String,
    /// The form found.
    pub form: PropertiesForm,
    /// 1-based line of the first Properties block, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// The advisory diagnostic, for a legacy form.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<LegacyFormDiagnostic>,
}

/// A block inside the Properties section.
#[derive(Debug, Clone, PartialEq, Eq)]
enum BlockKind {
    Table { header: Vec<String> },
    List,
    SysmlFence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Block {
    kind: BlockKind,
    line: usize,
}

/// What [`classify_properties`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    /// The form of the first block, or `None`.
    pub form: PropertiesForm,
    /// 1-based line — of the first block when there is one, of the
    /// `## Properties` heading when the section is empty, absent when there is
    /// no section at all.
    pub line: Option<usize>,
}

/// Split a Markdown table row into trimmed cells.
fn parse_header(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

/// True for `## Properties` with nothing but whitespace after it.
fn is_properties_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("##") else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(|c: char| c == ' ' || c == '\t') else {
        return false;
    };
    let rest = rest.trim_start_matches([' ', '\t']);
    let Some(rest) = rest.strip_prefix("Properties") else {
        return false;
    };
    rest.chars().all(char::is_whitespace)
}

/// True for `##` followed by whitespace — any level-2-or-deeper heading.
fn is_section_break(line: &str) -> bool {
    line.strip_prefix("##")
        .is_some_and(|rest| rest.starts_with([' ', '\t']))
}

/// A fence opener's info string, when the line opens one.
fn fence_language(line: &str) -> Option<String> {
    let rest = line.strip_prefix("```")?;
    let language: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '_' | '-'))
        .collect();
    Some(language)
}

/// True for a table separator row: optional pipe, optional colon, then `---`.
fn is_separator_row(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix('|').unwrap_or(rest);
    let rest = rest.trim_start_matches([' ', '\t']);
    let rest = rest.strip_prefix(':').unwrap_or(rest);
    rest.starts_with("---")
}

/// True for a bullet-list item: whitespace, `-` or `*`, whitespace, non-space.
fn is_list_item(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    let Some(rest) = rest.strip_prefix(['-', '*']) else {
        return false;
    };
    let after = rest.trim_start_matches([' ', '\t']);
    rest.starts_with([' ', '\t']) && !after.is_empty()
}

/// Find the Properties section (a `## Properties` heading up to the next `##`)
/// and classify its first block.
#[must_use]
pub fn classify_properties(markdown: &str) -> Classification {
    // `String.prototype.split("\n")` — a `\r` stays on the end of each line, so
    // the trimming below has to tolerate it exactly where the TypeScript's
    // regexes do.
    let lines: Vec<&str> = markdown.split('\n').collect();
    let Some(start) = lines
        .iter()
        .position(|line| is_properties_heading(line.trim_end_matches('\r')))
    else {
        return Classification {
            form: PropertiesForm::None,
            line: None,
        };
    };

    let mut blocks: Vec<Block> = Vec::new();
    let mut in_fence = false;
    let mut index = start + 1;
    while let Some(raw) = lines.get(index) {
        let text = raw.trim_end_matches('\r');
        if is_section_break(text) && !in_fence {
            break;
        }
        if in_fence {
            if text.starts_with("```") {
                in_fence = false;
            }
            index += 1;
            continue;
        }
        if let Some(language) = fence_language(text) {
            in_fence = true;
            if language == "sysml" {
                blocks.push(Block {
                    kind: BlockKind::SysmlFence,
                    line: index + 1,
                });
            }
            index += 1;
            continue;
        }
        if text.trim_start_matches([' ', '\t']).starts_with('|') {
            let next = lines
                .get(index + 1)
                .map_or("", |l| l.trim_end_matches('\r'));
            if is_separator_row(next) {
                blocks.push(Block {
                    kind: BlockKind::Table {
                        header: parse_header(text),
                    },
                    line: index + 1,
                });
            }
            index += 1;
            continue;
        }
        if is_list_item(text) {
            let continues = matches!(
                blocks.last(),
                Some(Block {
                    kind: BlockKind::List,
                    ..
                })
            );
            if !continues {
                blocks.push(Block {
                    kind: BlockKind::List,
                    line: index + 1,
                });
            }
        }
        index += 1;
    }

    let Some(first) = blocks.first() else {
        return Classification {
            form: PropertiesForm::None,
            line: Some(start + 1),
        };
    };
    let form = match &first.kind {
        BlockKind::SysmlFence => PropertiesForm::SysmlFence,
        BlockKind::List => PropertiesForm::BulletList,
        BlockKind::Table { header } => {
            if header.len() == TYPED_HEADER.len()
                && header
                    .iter()
                    .zip(TYPED_HEADER.iter())
                    .all(|(cell, expected)| cell == expected)
            {
                PropertiesForm::TypedTable
            } else {
                PropertiesForm::FreeColumnTable
            }
        }
    };
    Classification {
        form,
        line: Some(first.line),
    }
}

/// Classify one artifact and, for a legacy form, attach the FR-074 warning.
#[must_use]
pub fn classify_artifact(path: &str, markdown: &str) -> FormFinding {
    let Classification { form, line } = classify_properties(markdown);
    let diagnostic = match (form.is_legacy(), line) {
        (true, Some(line)) => Some(LegacyFormDiagnostic {
            code: "semantic.legacy-properties-form",
            severity: "warning",
            form,
            line,
            migration: "typed-table",
        }),
        _ => None,
    };
    FormFinding {
        path: path.to_owned(),
        form,
        line,
        diagnostic,
    }
}

/// One corpus root to sweep.
#[derive(Debug, Clone)]
pub struct CorpusRoot {
    /// The directory to walk.
    pub root: PathBuf,
    /// The repository the root stands for, used as the finding-path prefix.
    pub repository: String,
    /// The revision swept, recorded in the report.
    pub revision: String,
}

/// The package and version a sweep report is for.
#[derive(Debug, Clone)]
pub struct SweepIdentity {
    /// `<org>/<repo>`.
    pub package: PackageIdentity,
    /// The module version.
    pub version: String,
}

/// A corpus root as it appears in the report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ReportCorpusRoot {
    /// `<org>/<repo>`.
    pub repository: String,
    /// The revision swept.
    pub revision: String,
}

/// The per-form tallies.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SweepCounts {
    /// How many artifacts were classified.
    pub artifacts: usize,
    /// One count per form.
    pub forms: BTreeMap<&'static str, usize>,
    /// The two legacy forms, repeated so the schema's `legacy` block is filled.
    pub legacy: BTreeMap<&'static str, usize>,
}

/// The document `semantic.sweep_report` points at.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SweepReport {
    /// `<org>/<repo>`.
    pub package: String,
    /// The module version.
    pub version: String,
    /// RFC 3339 UTC, as the schema's pattern requires.
    #[serde(rename = "generatedAt")]
    pub generated_at: String,
    /// The roots swept.
    pub corpus: Vec<ReportCorpusRoot>,
    /// The tallies.
    pub counts: SweepCounts,
    /// One entry per artifact.
    pub findings: Vec<FormFinding>,
}

/// Every `*.md` under `root`, in the TypeScript's walk order: entries sorted per
/// directory, `node_modules` and dot-prefixed entries skipped, directories
/// recursed in place.
///
/// # Errors
///
/// [`crate::SemanticError::CorpusUnreadable`] when a directory cannot be listed.
pub fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, crate::SemanticError> {
    let mut out = Vec::new();
    walk_markdown(root, &mut out)?;
    Ok(out)
}

fn walk_markdown(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), crate::SemanticError> {
    let entries =
        std::fs::read_dir(root).map_err(|source| crate::SemanticError::CorpusUnreadable {
            path: root.to_path_buf(),
            source,
        })?;
    let mut names: Vec<String> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| crate::SemanticError::CorpusUnreadable {
            path: root.to_path_buf(),
            source,
        })?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    for name in names {
        if name == "node_modules" || name.starts_with('.') {
            continue;
        }
        let full = root.join(&name);
        let metadata =
            std::fs::metadata(&full).map_err(|source| crate::SemanticError::CorpusUnreadable {
                path: full.clone(),
                source,
            })?;
        if metadata.is_dir() {
            walk_markdown(&full, out)?;
        } else {
            // Case-sensitive on purpose: the TypeScript walks
            // `entry.endsWith(".md")`, and a case-insensitive match here would
            // sweep files the oracle's report never counted.
            #[allow(clippy::case_sensitive_file_extension_comparisons)]
            let is_markdown = name.ends_with(".md");
            if is_markdown {
                out.push(full);
            }
        }
    }
    Ok(())
}

/// Walk corpus roots and classify every Markdown artifact under them.
///
/// `generated_at` is supplied rather than read from the clock: a report whose
/// timestamp came from `SystemTime::now()` inside this function could not be
/// asserted, and the TypeScript takes the same parameter for the same reason.
///
/// # Errors
///
/// [`crate::SemanticError::CorpusUnreadable`] when a root cannot be walked, or
/// one of its files cannot be read.
pub fn sweep_corpus(
    roots: &[CorpusRoot],
    identity: &SweepIdentity,
    generated_at: &str,
) -> Result<SweepReport, crate::SemanticError> {
    let mut findings: Vec<FormFinding> = Vec::new();
    let mut forms: BTreeMap<&'static str, usize> = PropertiesForm::all()
        .iter()
        .map(|f| (f.as_str(), 0))
        .collect();

    for root in roots {
        for file in markdown_files(&root.root)? {
            let relative = file
                .strip_prefix(&root.root)
                .unwrap_or(&file)
                .to_string_lossy()
                .replace('\\', "/");
            let markdown = std::fs::read_to_string(&file).map_err(|source| {
                crate::SemanticError::CorpusUnreadable {
                    path: file.clone(),
                    source,
                }
            })?;
            let finding = classify_artifact(&format!("{}:{relative}", root.repository), &markdown);
            *forms.entry(finding.form.as_str()).or_insert(0) += 1;
            findings.push(finding);
        }
    }

    let legacy: BTreeMap<&'static str, usize> = [
        (
            PropertiesForm::BulletList.as_str(),
            forms
                .get(PropertiesForm::BulletList.as_str())
                .copied()
                .unwrap_or(0),
        ),
        (
            PropertiesForm::FreeColumnTable.as_str(),
            forms
                .get(PropertiesForm::FreeColumnTable.as_str())
                .copied()
                .unwrap_or(0),
        ),
    ]
    .into_iter()
    .collect();

    Ok(SweepReport {
        package: identity.package.as_str().to_owned(),
        version: identity.version.clone(),
        generated_at: generated_at.to_owned(),
        corpus: roots
            .iter()
            .map(|root| ReportCorpusRoot {
                repository: root.repository.clone(),
                revision: root.revision.clone(),
            })
            .collect(),
        counts: SweepCounts {
            artifacts: findings.len(),
            forms,
            legacy,
        },
        findings,
    })
}

/// The migration text the authoring pack shows once per semantic module.
pub const LEGACY_MIGRATION_EXAMPLE: &str = concat!(
    "Properties migration (FR-074): the typed table is the authored form.\n",
    "  before:  | Column | Type | Constraints |\n",
    "           | id | UUID | PK |\n",
    "  after:   | Field | Type | Multiplicity | Constraints |\n",
    "           | id | UUID | 1 | identity |\n",
    "  Legacy bullet lists and free-column tables validate at warning until the\n",
    "  module records a sweep report and sets semantic.legacy_forms: error."
);

#[cfg(test)]
// Indexing and `unreachable!` are a test-only convenience: an out-of-range
// index in a test is a failing test, not a downed worker.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    /// Trace: FR-074
    #[test]
    fn tc_378_080_properties_heading_must_be_exactly_level_two_and_bare() {
        assert!(is_properties_heading("## Properties"));
        assert!(is_properties_heading("## Properties  "));
        assert!(is_properties_heading("##  Properties"));
        assert!(!is_properties_heading("### Properties"));
        assert!(!is_properties_heading("## Properties (draft)"));
        assert!(!is_properties_heading("##Properties"));
        assert!(!is_properties_heading("# Properties"));
    }

    /// Trace: FR-074
    #[test]
    fn tc_378_081_separator_row_accepts_alignment_colons() {
        assert!(is_separator_row("| --- |"));
        assert!(is_separator_row("|:---|"));
        assert!(is_separator_row("---"));
        assert!(!is_separator_row("| -- |"));
        assert!(!is_separator_row("| id |"));
    }

    /// Trace: FR-074
    #[test]
    fn tc_378_082_list_item_needs_a_space_and_text() {
        assert!(is_list_item("- id"));
        assert!(is_list_item("  * id"));
        assert!(!is_list_item("-"));
        assert!(!is_list_item("-id"));
        assert!(!is_list_item("- "));
    }

    /// Trace: FR-074
    #[test]
    fn tc_378_083_legacy_forms_are_exactly_two() {
        let legacy: Vec<_> = PropertiesForm::all()
            .iter()
            .filter(|f| f.is_legacy())
            .collect();
        assert_eq!(
            legacy,
            vec![
                &PropertiesForm::FreeColumnTable,
                &PropertiesForm::BulletList
            ]
        );
    }

    /// Trace: FR-074
    #[test]
    fn tc_378_084_migration_example_names_both_forms() {
        assert!(LEGACY_MIGRATION_EXAMPLE.contains("Field | Type | Multiplicity | Constraints"));
        assert!(LEGACY_MIGRATION_EXAMPLE.contains("legacy_forms: error"));
    }
}
