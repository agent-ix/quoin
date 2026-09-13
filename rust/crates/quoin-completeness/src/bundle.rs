// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Reading a bundle's declared-vocabulary claims (FR-037).
//!
//! **On quoin reading documents at all.** quire is the parser, and quoin does
//! not reimplement it — every structural question about a document goes to
//! `quire validate` / `extract` / `coverage`. What is read here is narrower: the
//! leading `---` block, as YAML, plus the raw body text.
//!
//! The alternative was one `quire extract` subprocess per document. `extract`
//! takes a single `<DOC>` and a `--module`, so a bundle sweep is N spawns to
//! answer a question about a handful of frontmatter keys. That is the wrong
//! trade for a check meant to run in CI.
//!
//! The better fix is upstream and is filed: if the FR-059 diagnostic classified
//! each value as owned / excused / unowned, quoin would need no reader at all.
//! Until then this stays deliberately dumb — no archetype resolution, no schema
//! validation, no link walking. If the frontmatter does not parse, the document
//! contributes nothing and says so.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::assess::DocumentClaims;
use crate::declarations::VocabularyDeclaration;

/// One document as the CALLER read it: a bundle-relative path and raw bytes.
///
/// The boundary's reason for existing (quoin#445). `quoin-core`'s library half
/// is audited as a `ReusableLibrary` by
/// `quoin-core/tests/tc_library_containment.rs`, so an operation that takes a
/// path and reads the disk fails the gate. The walk and the reads stay in the
/// command shell — which is where a CLI's I/O belongs — and the decision
/// crosses the boundary as bytes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DocumentSource {
    /// Path, relative to the bundle root, with `/` separators.
    pub path: String,
    /// The whole file, as text.
    pub raw: String,
}

/// A document whose frontmatter could not be read, and why.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UnreadableDocument {
    /// Path, relative to the bundle root, with `/` separators.
    pub path: String,
    /// Why it could not be read.
    pub reason: String,
}

/// One document's frontmatter and body, as read from the bundle.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct BundleDocument {
    /// Path, relative to the bundle root, with `/` separators.
    pub path: String,
    /// The parsed leading `---` block.
    pub frontmatter: serde_json::Map<String, Value>,
    /// Everything after it.
    pub body: String,
}

/// Every document under a bundle root that carries parseable frontmatter.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FrontmatterRead {
    /// The documents.
    pub documents: Vec<BundleDocument>,
    /// Documents whose frontmatter could not be parsed.
    pub unreadable: Vec<UnreadableDocument>,
}

/// One declaration's projection over a bundle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BundleRead {
    /// Documents that claim or excuse at least one value.
    pub documents: Vec<DocumentClaims>,
    /// Documents whose frontmatter could not be parsed.
    pub unreadable: Vec<UnreadableDocument>,
}

/// Split `---\n…\n---\n` at the head of a file from the body after it.
///
/// The TypeScript regex is
/// `^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$`, anchored at the start and
/// non-greedy, so the **first** closing `---` wins and a file not starting with
/// `---` has no frontmatter at all.
fn split_frontmatter(raw: &str) -> Option<(&str, &str)> {
    let after_open = raw
        .strip_prefix("---\r\n")
        .or_else(|| raw.strip_prefix("---\n"))?;

    let mut search = 0usize;
    while let Some(found) = after_open.get(search..)?.find("---") {
        let close = search + found;
        // The regex requires `\r?\n` immediately before the closing fence.
        let before = after_open.get(..close)?;
        let yaml = if let Some(rest) = before.strip_suffix("\r\n") {
            Some(rest)
        } else {
            before.strip_suffix('\n')
        };
        if let Some(yaml) = yaml {
            let after_close = after_open.get(close + 3..)?;
            let body = after_close
                .strip_prefix("\r\n")
                .or_else(|| after_close.strip_prefix('\n'))
                .unwrap_or(after_close);
            return Some((yaml, body));
        }
        search = close + 3;
    }
    None
}

/// Every document under `bundle_root` that carries parseable frontmatter.
///
/// One pass and one reader, shared by the completeness sweep (FR-037) and the
/// assurance view (FR-040). Two readers over the same files would drift, and the
/// second would be written by whoever needed a field the first did not expose.
///
/// An absent bundle root reads as an empty bundle: the command reports the root
/// it looked in, so this is legible rather than mysterious.
#[must_use]
pub fn read_bundle_frontmatter(bundle_root: &Path) -> FrontmatterRead {
    let mut read = FrontmatterRead::default();

    for path in markdown_under(bundle_root) {
        let relative = path
            .strip_prefix(bundle_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        match std::fs::read_to_string(&path) {
            // Absorbed inside the walk, not collected and parsed afterwards: an
            // OS read failure and a YAML failure are both `unreadable` entries,
            // and the order they are reported in is the walk order. Two passes
            // would group them by kind instead, which is a different list.
            Ok(raw) => absorb(
                &mut read,
                &DocumentSource {
                    path: relative,
                    raw,
                },
            ),
            Err(cause) => read.unreadable.push(UnreadableDocument {
                path: relative,
                reason: cause.to_string(),
            }),
        }
    }

    read
}

/// Parse frontmatter out of documents the CALLER read.
///
/// The decidable half of [`read_bundle_frontmatter`], split out so it can cross
/// the `quoin-core` boundary without the library half acquiring a filesystem
/// (quoin#445). The filesystem shell above is the only other caller, so there
/// is one parser and not two.
///
/// `unreadable` is what the CALLER could not open. It leads the list because
/// the filesystem shell interleaves both kinds in walk order and a wire caller
/// cannot express that interleaving — it can only send what it managed to read.
/// Dropping it instead would turn a broken bundle into a clean one.
#[must_use]
pub fn frontmatter_from_sources(
    documents: &[DocumentSource],
    unreadable: &[UnreadableDocument],
) -> FrontmatterRead {
    let mut read = FrontmatterRead {
        documents: Vec::new(),
        unreadable: unreadable.to_vec(),
    };
    for source in documents {
        absorb(&mut read, source);
    }
    read
}

/// Parse one document into `read`, as either a `BundleDocument` or an
/// `UnreadableDocument`, or as neither when it carries no frontmatter at all.
fn absorb(read: &mut FrontmatterRead, source: &DocumentSource) {
    // No frontmatter is not an error: an index or a README declares nothing.
    let Some((yaml, body)) = split_frontmatter(&source.raw) else {
        return;
    };
    let parsed: Value = match quoin_yaml::from_str(yaml) {
        Ok(value) => value,
        Err(cause) => {
            // Reported, not skipped silently: a document whose frontmatter does
            // not parse may be the one carrying the exclusion, and dropping it
            // would turn a broken declaration into a clean bundle.
            read.unreadable.push(UnreadableDocument {
                path: source.path.clone(),
                reason: cause.to_string(),
            });
            return;
        }
    };
    // `parseYaml(...) ?? {}` — an empty block yields null, which the TypeScript
    // coerces to an empty object. A scalar or sequence is neither and
    // contributes nothing.
    let frontmatter = match parsed {
        Value::Null => serde_json::Map::new(),
        Value::Object(map) => map,
        _ => return,
    };
    read.documents.push(BundleDocument {
        path: source.path.clone(),
        frontmatter,
        body: body.to_owned(),
    });
}

/// Project already-read documents onto one declaration.
///
/// Split out from [`read_bundle_claims`] so a caller with several declarations
/// walks the bundle **once**. The combined form re-read every file per
/// declaration, which NFR-011-M-2 budgets at one pass per invocation.
///
/// Both the claim field and the justified-absence field accept a scalar or a
/// list, because `quality_attribute: security` and
/// `quality_attributes_not_applicable: [safety, compliance]` are both ordinary
/// authoring — the same two shapes the engine accepts, for the same reason.
#[must_use]
pub fn claims_for(
    documents: &[BundleDocument],
    declaration: &VocabularyDeclaration,
) -> Vec<DocumentClaims> {
    let mut out = Vec::new();
    for document in documents {
        let claims = strings_at(document.frontmatter.get(declaration.field.as_str()));
        let excuses = declaration
            .justified_absence_field
            .as_deref()
            .map(|field| strings_at(document.frontmatter.get(field)))
            .unwrap_or_default();
        if claims.is_empty() && excuses.is_empty() {
            continue;
        }
        out.push(DocumentClaims {
            path: document.path.clone(),
            claims,
            excuses,
            body: document.body.clone(),
        });
    }
    out
}

/// Read every markdown document under `bundle_root` for one declaration's
/// fields.
#[must_use]
pub fn read_bundle_claims(bundle_root: &Path, declaration: &VocabularyDeclaration) -> BundleRead {
    let read = read_bundle_frontmatter(bundle_root);
    BundleRead {
        documents: claims_for(&read.documents, declaration),
        unreadable: read.unreadable,
    }
}

/// Every `*.md` under `root`, recursively, sorted by relative path.
///
/// The TypeScript recurses per directory and sorts the **relative** entries,
/// which is not the same order as sorting absolute paths per directory —
/// `a/b.md` sorts before `a-b.md` one way and after it the other. Sorting
/// relative paths keeps the reported order identical.
fn markdown_under(root: &Path) -> Vec<PathBuf> {
    let mut relative: Vec<String> = Vec::new();
    collect(root, Path::new(""), &mut relative);
    relative.sort();
    relative.into_iter().map(|entry| root.join(entry)).collect()
}

/// Is this relative path a markdown document by NAME?
///
/// Case-sensitive on purpose: the TypeScript filters `entry.endsWith(".md")`,
/// and a case-insensitive match here would read documents the oracle never saw.
#[allow(
    clippy::case_sensitive_file_extension_comparisons,
    reason = "the oracle's `endsWith('.md')` is case-sensitive; matching case-insensitively here \
              would admit documents the shell never puts on the wire"
)]
fn is_markdown_name(relative: &str) -> bool {
    relative.ends_with(".md")
}

fn collect(root: &Path, prefix: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(root.join(prefix)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let relative = prefix.join(&name);
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        // `read_dir` reports the LINK's own type, never its target's — so a
        // real directory is descended and a symlinked one is not, which is what
        // keeps the walk from following a cycle out of the bundle.
        if file_type.is_dir() {
            collect(root, &relative, out);
            continue;
        }
        let text = relative.to_string_lossy().replace('\\', "/");
        if !is_markdown_name(&text) {
            continue;
        }
        // A symlink is a document exactly when it RESOLVES to a file. That is
        // the deliberate decision at this seam, and the two sides state it
        // identically (`src/core/snapshot.ts` asks `statSync(...).isFile()`):
        //
        // - resolving to a FILE: a document. `spec/FR-042.md` symlinked into a
        //   shared spec directory is the normal monorepo layout, and dropping
        //   it loses that document's claims from the verdict entirely — the
        //   value it owned is reported `unowned` and a finding appears that a
        //   direct disk read never produces.
        // - resolving to a DIRECTORY: not a document, and not descended. A
        //   directory may be named `notes.md`; opening it invents an `EISDIR`
        //   entry for a file the bundle does not hold.
        // - resolving to NOTHING (a broken link): not a document. There are no
        //   bytes to read, and reporting it `unreadable` would be a document
        //   the tree does not contain.
        //
        // This differs from `quoin-validators`' repository walk, which drops
        // symlinks outright — that walk scans an arbitrary repository for build
        // wiring, where a link is far likelier to be an escape than content. A
        // completeness bundle is a curated document set whose links are the
        // point.
        let resolved_to_file = if file_type.is_symlink() {
            std::fs::metadata(root.join(&relative)).is_ok_and(|meta| meta.is_file())
        } else {
            file_type.is_file()
        };
        if resolved_to_file {
            out.push(text);
        }
    }
}

/// A frontmatter value as a list of strings, from either the scalar or list
/// form. Non-string members are dropped, as the TypeScript's type guard does.
fn strings_at(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-037
    #[test]
    fn tc_378_230_frontmatter_split_takes_the_first_closing_fence() {
        let raw = "---\na: 1\n---\nbody\n---\nmore\n";
        assert_eq!(split_frontmatter(raw), Some(("a: 1", "body\n---\nmore\n")));
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_231_frontmatter_split_handles_crlf() {
        let raw = "---\r\na: 1\r\n---\r\nbody\r\n";
        assert_eq!(split_frontmatter(raw), Some(("a: 1", "body\r\n")));
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_232_a_document_not_starting_with_a_fence_has_no_frontmatter() {
        assert_eq!(split_frontmatter("# Index\n\n---\na: 1\n---\n"), None);
        assert_eq!(split_frontmatter(""), None);
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_233_an_empty_frontmatter_block_is_an_empty_map_not_a_failure() {
        assert_eq!(split_frontmatter("---\n\n---\nbody"), Some(("", "body")));
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_234_scalar_and_list_claim_forms_both_read() {
        assert_eq!(
            strings_at(Some(&serde_json::json!("security"))),
            vec!["security".to_owned()]
        );
        assert_eq!(
            strings_at(Some(&serde_json::json!(["a", 1, "b"]))),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert!(strings_at(Some(&serde_json::json!(7))).is_empty());
        assert!(strings_at(None).is_empty());
    }
}
