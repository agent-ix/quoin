// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Assessing one bundle against every declared vocabulary (FR-037).
//!
//! One function so the command is a thin shell over it, and so the criteria
//! stated over `quoin completeness` and the unit assertions exercise the same
//! path rather than two that agree by inspection.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::assess::{
    CompletenessFinding, Verdict, VocabularyRollup, assess_vocabulary, verdict_for,
};
use crate::bundle::{UnreadableDocument, claims_for, read_bundle_frontmatter};
use crate::declarations::{UnresolvedDeclaration, load_vocabulary_coverage};
use crate::ids::VocabularyName;

/// What to assess, and how strictly.
#[derive(Debug, Clone)]
pub struct AssessOptions {
    /// Bundle root — the directory whose documents are read.
    pub bundle_root: PathBuf,
    /// Promote an admitted gap to a failing verdict.
    pub strict: bool,
    /// Module roots to read declarations from.
    ///
    /// The TypeScript defaults this to `defaultModuleRoots()`, which reads
    /// `QUOIN_MODULE_PATHS` and the installed module directory. That resolution
    /// belongs to the catalog (EPIC #373 Stage 7) and is **not** ported here:
    /// this crate would otherwise carry a second answer to "where are the
    /// modules", which is exactly the duplication FR-037 exists to avoid. The
    /// caller supplies the set.
    pub module_roots: Vec<PathBuf>,
}

/// The report the command prints.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BundleAssessment {
    /// The root that was read.
    #[serde(rename = "bundleRoot")]
    pub bundle_root: PathBuf,
    /// Declarations found, so a report of zero findings can be told from zero
    /// checks.
    pub vocabularies: Vec<VocabularyName>,
    /// Declarations whose vocabulary could not be resolved.
    pub unresolved: Vec<UnresolvedDeclaration>,
    /// Documents whose frontmatter could not be parsed.
    pub unreadable: Vec<UnreadableDocument>,
    /// One tally per declaration.
    pub rollups: Vec<VocabularyRollup>,
    /// Every gap found, sorted.
    pub findings: Vec<CompletenessFinding>,
    /// The verdict.
    pub verdict: Verdict,
}

/// Assess a bundle and return the report the command prints.
///
/// **Zero declarations is reported, not passed.** A repository whose module set
/// declares no vocabulary coverage has not been checked, and printing `PASS`
/// over it would be the "green matrix over dead links" result this program was
/// created to stop. `vocabularies: []` makes the difference visible in both the
/// human and JSON output.
#[must_use]
pub fn assess_bundle(options: &AssessOptions) -> BundleAssessment {
    let loaded = load_vocabulary_coverage(&options.module_roots);
    let mut findings: Vec<CompletenessFinding> = Vec::new();
    let mut rollups: Vec<VocabularyRollup> = Vec::new();
    let mut unreadable: Vec<UnreadableDocument> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();

    // ONE walk, whatever the declaration count. `read_bundle_claims` re-reads
    // the whole bundle per declaration, so N declarations would mean N full
    // passes — and NFR-011-M-2 states the budget as one pass per invocation.
    let bundle = read_bundle_frontmatter(&options.bundle_root);
    for entry in bundle.unreadable {
        if seen.insert(entry.path.clone()) {
            unreadable.push(entry);
        }
    }

    for declaration in &loaded.declarations {
        let assessed = assess_vocabulary(declaration, &claims_for(&bundle.documents, declaration));
        rollups.push(assessed.rollup);
        findings.extend(assessed.findings);
    }

    // `localeCompare` on the ASCII identifiers these fields carry agrees with
    // byte order. `sort` in JavaScript is stable, and `sort_by` here is too, so
    // two findings identical on all three keys keep their discovery order.
    findings.sort_by(|a, b| {
        a.vocabulary
            .as_str()
            .cmp(b.vocabulary.as_str())
            .then_with(|| a.kind.as_str().cmp(b.kind.as_str()))
            .then_with(|| a.value.as_str().cmp(b.value.as_str()))
    });

    let verdict = verdict_for(&findings, options.strict, loaded.declarations.len());

    BundleAssessment {
        bundle_root: options.bundle_root.clone(),
        vocabularies: loaded.declarations.iter().map(|d| d.name.clone()).collect(),
        unresolved: loaded.unresolved,
        unreadable,
        rollups,
        findings,
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trace: FR-037
    #[test]
    fn tc_378_240_a_bundle_with_no_declarations_is_unchecked_not_passed() {
        let Ok(temp) = tempfile::tempdir() else {
            unreachable!("tempdir");
        };
        let assessment = assess_bundle(&AssessOptions {
            bundle_root: temp.path().to_path_buf(),
            strict: false,
            module_roots: Vec::new(),
        });
        assert_eq!(assessment.verdict, Verdict::Unchecked);
        assert!(assessment.vocabularies.is_empty());
    }

    /// Trace: FR-037
    #[test]
    fn tc_378_241_an_absent_bundle_root_reads_as_empty_not_as_a_panic() {
        let assessment = assess_bundle(&AssessOptions {
            bundle_root: PathBuf::from("/definitely/not/here"),
            strict: false,
            module_roots: Vec::new(),
        });
        assert!(assessment.unreadable.is_empty());
        assert_eq!(assessment.verdict, Verdict::Unchecked);
    }
}
