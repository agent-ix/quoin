// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Formalized QSL declarations and their source spans (VER-101).

use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use crate::{Error, Result};

/// The engine-declared model used by QSL relation extraction.
pub use quire_rs::traceability::TraceabilityModel;

/// One QSL declaration that carries at least one `formalizes` relation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalRule {
    /// Stable QSL symbol id, `qsl:<path>::<label>`.
    pub symbol: String,
    /// Repo-relative source path.
    pub path: String,
    /// QSL declaration label.
    pub label: String,
    /// Requirement and criterion ids named by the declaration's markers.
    pub formalizes: Vec<String>,
    /// Half-open UTF-8 byte offsets of the declaration itself in its file.
    pub declaration_range: Range<usize>,
    /// The matching marker comment(s), intervening allowed comment/blank lines,
    /// and declaration span, in their original UTF-8 bytes.
    pub source: Vec<u8>,
}

/// Extract formalized QSL declarations with the engine's QSL adapter and the
/// repository's declared traceability model.
///
/// The returned `source` is suitable for byte comparison across git revisions;
/// it is not a normalized or reconstructed rendering.
///
/// # Errors
///
/// Returns an error if the QSL adapter cannot retain a declaration's source or
/// reports a source span outside the retained file.
pub fn extract(root: &Path, model: &TraceabilityModel) -> Result<Vec<FormalRule>> {
    let extraction = quire_rs::symbols::extract_tree(root);
    let graph = quire_rs::symbols::trace::bind(&extraction, model);
    let mut by_symbol: BTreeMap<String, (Vec<String>, usize)> = BTreeMap::new();
    for relation in graph.formalizes {
        let entry = by_symbol
            .entry(relation.symbol_id)
            .or_insert_with(|| (Vec::new(), relation.line));
        entry.0.push(relation.trace_id);
        entry.1 = entry.1.min(relation.line);
    }

    let mut rules = Vec::with_capacity(by_symbol.len());
    for symbol in &extraction.symbols {
        let Some((formalizes, marker_line)) = by_symbol.remove(&symbol.id) else {
            continue;
        };
        let Some(source) = extraction.source_of(&symbol.path) else {
            return Err(Error::Io {
                path: root.join(&symbol.path),
                reason: "QSL adapter did not retain source for a formalized declaration".to_owned(),
            });
        };
        let declaration_range = declaration_span(
            source.as_bytes(),
            symbol.line,
            symbol.column,
            symbol.end_line,
        )
        .ok_or_else(|| Error::Io {
            path: root.join(&symbol.path),
            reason: "QSL adapter returned a declaration span outside the retained file".to_owned(),
        })?;
        let bytes =
            line_span(source.as_bytes(), marker_line, symbol.end_line).ok_or_else(|| {
                Error::Io {
                    path: root.join(&symbol.path),
                    reason: "QSL adapter returned a source span outside the retained file"
                        .to_owned(),
                }
            })?;
        rules.push(FormalRule {
            // VER-101's signed trailer contract uses the public QSL symbol
            // spelling also used by the workflow: qsl:<path>::<label>.
            // Keep quire-rs' structured symbol ID internal for joining graph
            // relations to declarations.
            symbol: format!("qsl:{}::{}", symbol.path, symbol.qualified_name),
            path: symbol.path.clone(),
            label: symbol.qualified_name.clone(),
            formalizes,
            declaration_range,
            source: bytes.to_vec(),
        });
    }
    rules.sort_by(|a, b| (&a.path, &a.label).cmp(&(&b.path, &b.label)));
    Ok(rules)
}

fn declaration_span(
    source: &[u8],
    start_line: usize,
    start_column: usize,
    end_line: usize,
) -> Option<Range<usize>> {
    if start_line == 0 || start_column == 0 || end_line < start_line {
        return None;
    }
    let mut line = 1usize;
    let mut line_start = 0usize;
    let mut start = None;
    let mut end = None;
    for (index, byte) in source.iter().enumerate() {
        if line == start_line && start.is_none() {
            start = Some(line_start.checked_add(start_column - 1)?);
        }
        if *byte == b'\n' {
            if line == end_line {
                end = Some(
                    if index > line_start
                        && index
                            .checked_sub(1)
                            .and_then(|previous| source.get(previous))
                            .is_some_and(|byte| *byte == b'\r')
                    {
                        index - 1
                    } else {
                        index
                    },
                );
                break;
            }
            line += 1;
            line_start = index + 1;
        }
    }
    if end.is_none() && line == end_line {
        end = Some(source.len());
    }
    let start = start?;
    let end = end?;
    (start < end && end <= source.len()).then_some(start..end)
}

fn line_span(source: &[u8], start: usize, end: usize) -> Option<&[u8]> {
    if start == 0 || end < start {
        return None;
    }
    let mut line = 1usize;
    let mut start_byte = None;
    for (index, byte) in source.iter().enumerate() {
        if line == start && start_byte.is_none() {
            start_byte = Some(index);
        }
        if *byte == b'\n' {
            if line == end {
                return source.get(start_byte?..=index);
            }
            line += 1;
        }
    }
    if line == start && start_byte.is_none() {
        start_byte = Some(source.len());
    }
    let start_byte = start_byte?;
    if line == end {
        source.get(start_byte..)
    } else {
        None
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "fixture assertions are test failures"
)]
mod tests {
    use super::{TraceabilityModel, declaration_span, extract, line_span};
    use std::{fs, path::Path};

    #[test]
    fn source_span_preserves_newline_bytes() {
        let source = b"first\r\nmarker\r\nrule P1 {\r\n}\r\n";
        assert_eq!(
            line_span(source, 2, 4),
            Some(&b"marker\r\nrule P1 {\r\n}\r\n"[..])
        );
        assert_eq!(line_span(b"one\ntwo", 2, 2), Some(&b"two"[..]));
        assert_eq!(line_span(source, 0, 2), None);
    }

    #[test]
    fn declaration_range_uses_utf8_byte_offsets_and_excludes_marker_lines() {
        let source = "// é before declaration\nrule P1 { check x }\n";
        let declaration_start = source.find("rule P1").expect("declaration start");
        let declaration_end = source.len() - 1;
        let range = declaration_span(source.as_bytes(), 2, 1, 2).expect("declaration range");
        assert_eq!(range, declaration_start..declaration_end);
        assert_eq!(&source.as_bytes()[range], b"rule P1 { check x }");
    }

    #[test]
    fn extraction_uses_the_qsl_formalizes_relation_and_retains_exact_span() {
        let root = tempfile::tempdir().expect("temporary source root");
        fs::write(
            root.path().join("a.native"),
            b"language qsl\nprofile core\n// formalizes: FR-001, FR-001-AC-2\nrule P1 {\n  check x\n}\n",
        )
        .expect("QSL source");
        let model: TraceabilityModel = serde_json::from_value(serde_json::json!({
            "trace_tags": {
                "formalizes": [{
                    "name": "formalizes",
                    "language": "qsl",
                    "syntax": "comment",
                    "pattern": "^// formalizes: (.+)$"
                }]
            }
        }))
        .expect("traceability model");
        let rules = extract(root.path(), &model).expect("QSL rules");
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].symbol, "qsl:a.native::P1");
        assert_eq!(rules[0].formalizes, ["FR-001", "FR-001-AC-2"]);
        assert_eq!(
            &b"language qsl\nprofile core\n// formalizes: FR-001, FR-001-AC-2\nrule P1 {\n  check x\n}\n"[
                rules[0].declaration_range.clone()
            ],
            b"rule P1 {\n  check x\n}"
        );
        assert_eq!(
            rules[0].source,
            b"// formalizes: FR-001, FR-001-AC-2\nrule P1 {\n  check x\n}\n"
        );
        assert!(Path::new(&rules[0].path).is_relative());
    }
}
