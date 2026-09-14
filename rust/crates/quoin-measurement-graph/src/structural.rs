// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Turning one resolved mapping into the opaque FR-062 structural graph the
//! projection is injected with.
//!
//! Ports `loadStructuralGraph` and `pathFor`
//! (`src/measurement/graph-portfolio-load.ts:95-161`), quoin#477.
//!
//! # The analyses are not this crate's
//!
//! [`quoin_graph_analysis`] owns the three views and the four reads behind
//! them (quoin#385). Nothing here computes a fan-out row, parses an export or
//! names a file format; the whole module is *which* refusal a caller sees, and
//! where the bytes cross into [`JsonValue`] — which is
//! [`InjectedStructuralGraph`]'s contract: the portfolio carries graph results
//! and interprets none of them.
//!
//! # `missing` is a `NotFound`, not a regular expression
//!
//! `graph-portfolio-load.ts:117` separates a missing export from an unreadable
//! one with `/ENOENT|no such file/i.test(loaded.error.message)` — a decision
//! taken by matching the prose of whichever `fs` error node happened to
//! produce. [`RecordingReader`] keeps the [`std::io::ErrorKind`] the seam
//! already reported, so the same distinction is read off a type. See
//! `DIVERGENCE.md` §9 for the one input on which the two disagree.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use quoin_graph_analysis::{
    ArtifactId, GraphAnalysisInput, GraphError, GraphInput, GraphInputReader, GraphLoadOptions,
    OsGraphInputReader, analyze_change_impact, analyze_churn, analyze_fan_out,
    load_graph_analysis_input,
};
use quoin_store::JsonValue;
use serde::Serialize;

use crate::assurance::adapt::adapt_quire_assurance;
use crate::assurance::premise::AcceptedQuirePremises;
use crate::availability::GapAvailability;
use crate::error::{GraphAdapterError, GraphAdapterErrorCode, Result};
use crate::input::InjectedStructuralGraph;
use crate::mapping::ResolvedMapping;

/// The four values a mapping that named all three documents carries.
///
/// The retained loader narrows with `mapping.status !== "ready"` and then
/// reads four members off the narrowed type; this is that narrowing, so the
/// functions below take what they need rather than re-matching the enum.
struct Ready<'a> {
    root: &'a Path,
    export: &'a Path,
    premises: &'a Path,
    audit: &'a Path,
    changed: &'a [String],
}

impl<'a> Ready<'a> {
    fn of(mapping: &'a ResolvedMapping) -> Option<Self> {
        match *mapping {
            ResolvedMapping::Ready {
                ref root,
                ref export_path,
                ref premises_path,
                ref audit_path,
                ref changed,
            } => Some(Self {
                root,
                export: export_path,
                premises: premises_path,
                audit: audit_path,
                changed,
            }),
            ResolvedMapping::Refused { .. } => None,
        }
    }

    /// `pathFor` (`graph-portfolio-load.ts:151-161`), including its `else`.
    const fn path_for(&self, input: Option<GraphInput>) -> &'a Path {
        match input {
            Some(GraphInput::Export) => self.export,
            Some(GraphInput::Premises) => self.premises,
            // `pathFor`'s trailing `return mapping.auditPath` is the
            // fall-through arm there and is the fall-through arm here: the
            // bindings store and a canonicalization refusal both arise while
            // the audit envelope is being read.
            _ => self.audit,
        }
    }
}

/// A [`GraphInputReader`] that remembers what each read answered.
///
/// The record is not a cache for speed. It holds the two facts the retained
/// loader recovers from prose afterwards: whether a failed read failed because
/// nothing was there, and what the export's bytes were — which
/// `graph-portfolio-load.ts:132` needs again for the FR-066 adapter handoff
/// and would otherwise take from a second read of a file that may have changed
/// between the two.
struct RecordingReader<R> {
    inner: R,
    seen: RefCell<BTreeMap<PathBuf, std::result::Result<String, ErrorKind>>>,
}

impl<R: GraphInputReader> RecordingReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            seen: RefCell::new(BTreeMap::new()),
        }
    }

    /// Whether the read of `path` failed because nothing was there.
    fn was_absent(&self, path: &Path) -> bool {
        matches!(
            self.seen.borrow().get(path),
            Some(&Err(ErrorKind::NotFound))
        )
    }

    /// The text one path read as, when it read at all.
    fn text(&self, path: &Path) -> Option<String> {
        self.seen
            .borrow()
            .get(path)
            .and_then(|held| held.clone().ok())
    }
}

impl<R: GraphInputReader> GraphInputReader for RecordingReader<R> {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        let answer = self.inner.read(path);
        self.seen.borrow_mut().insert(
            path.to_path_buf(),
            match answer {
                Ok(ref text) => Ok(text.clone()),
                Err(ref error) => Err(error.kind()),
            },
        );
        answer
    }
}

/// The structural graph for one mapping, read from the real filesystem.
///
/// `loadStructuralGraph` (`graph-portfolio-load.ts:95-149`). Every refusal is
/// a value: this returns no `Err`, because a repository whose graph cannot be
/// read is a repository with a stated gap, not a failed portfolio.
#[must_use]
pub fn load_structural_graph(mapping: &ResolvedMapping) -> InjectedStructuralGraph {
    load_structural_graph_with(OsGraphInputReader, mapping)
}

/// [`load_structural_graph`] over a stated reader.
#[must_use]
pub fn load_structural_graph_with<R: GraphInputReader>(
    reader: R,
    mapping: &ResolvedMapping,
) -> InjectedStructuralGraph {
    let Some(ready) = Ready::of(mapping) else {
        // `graph-portfolio-load.ts:98-99`: a refused mapping is its own answer
        // and no file is opened for it.
        return InjectedStructuralGraph::Unavailable {
            availability: match *mapping {
                ResolvedMapping::Refused { status, .. } => status.into(),
                ResolvedMapping::Ready { .. } => GapAvailability::Unknown,
            },
            path: None,
            reason: mapping.reason().unwrap_or_default().to_owned(),
        };
    };

    let reader = RecordingReader::new(reader);
    let loaded = match load_graph_analysis_input(
        &reader,
        &GraphLoadOptions {
            repo: ready.root.to_path_buf(),
            export_path: ready.export.to_path_buf(),
            premises_path: ready.premises.to_path_buf(),
            audit_path: ready.audit.to_path_buf(),
        },
    ) {
        Ok(loaded) => loaded,
        Err(error) => {
            let path = ready.path_for(error.input());
            return InjectedStructuralGraph::Unavailable {
                availability: refused_availability(&error, &reader, path),
                path: Some(path.to_string_lossy().into_owned()),
                reason: error.to_string(),
            };
        }
    };

    // `graph-portfolio-load.ts:129-157`: the adapter handoff and the analyses
    // are inside one `try`, and anything thrown there is `incompatible`.
    admitted(&reader, &loaded, &ready)
        .and_then(|graph| with_change_impact(graph, ready.changed, &loaded))
        .unwrap_or_else(|error| InjectedStructuralGraph::Unavailable {
            availability: GapAvailability::Incompatible,
            path: Some(ready.export.to_string_lossy().into_owned()),
            reason: error.to_string(),
        })
}

/// Everything `graph-portfolio-load.ts:130-143` does once the three inputs are
/// read: the FR-066 adapter handoff, then the two analyses that always run.
fn admitted<R: GraphInputReader>(
    reader: &RecordingReader<R>,
    loaded: &GraphAnalysisInput,
    ready: &Ready<'_>,
) -> Result<InjectedStructuralGraph> {
    // FR-066's adapter performs the same closed producer-contract handoff used
    // by other consumers; FR-062 remains the sole owner of graph semantics.
    let text = reader.text(ready.export).ok_or_else(|| {
        refused(format!(
            "{}: the export was not read",
            ready.export.display()
        ))
    })?;
    let document: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| refused(error.to_string()))?;
    adapt_quire_assurance(&document, &accepted_premises(loaded)?)?;

    Ok(InjectedStructuralGraph::Available {
        path: Some(ready.export.to_string_lossy().into_owned()),
        premises: opaque(&loaded.premises)?,
        fan_out: opaque(&analyze_fan_out(loaded))?,
        churn: opaque(&analyze_churn(loaded))?,
        change_impact: None,
    })
}

/// `...(mapping.changed.length > 0 ? { changeImpact: [ … ] } : {})`
/// (`graph-portfolio-load.ts:144-146`).
fn with_change_impact(
    graph: InjectedStructuralGraph,
    changed: &[String],
    loaded: &GraphAnalysisInput,
) -> Result<InjectedStructuralGraph> {
    let InjectedStructuralGraph::Available {
        path,
        premises,
        fan_out,
        churn,
        change_impact: _,
    } = graph
    else {
        return Ok(graph);
    };
    let analyses = if changed.is_empty() {
        None
    } else {
        let seeds: Vec<ArtifactId> = changed.iter().map(ArtifactId::new).collect();
        let analysis = analyze_change_impact(loaded, &seeds, None)
            .map_err(|error| refused(error.to_string()))?;
        Some(vec![opaque(&analysis)?])
    };
    Ok(InjectedStructuralGraph::Available {
        path,
        premises,
        fan_out,
        churn,
        change_impact: analyses,
    })
}

/// The premises the FR-066 adapter is asked to admit the export under.
///
/// `graph-portfolio-load.ts:135-140` builds this from the export's own source
/// and the premises document's modules. Both halves are re-read through
/// [`AcceptedQuirePremises`]'s own contract rather than transcribed member by
/// member, so the scalars this crate already proved — `FullRevision`,
/// `BareDigest` — are the ones the adapter compares.
fn accepted_premises(loaded: &GraphAnalysisInput) -> Result<AcceptedQuirePremises> {
    let stated = serde_json::json!({
        "source": loaded.assurance.source,
        "modules": loaded.premises.modules,
    });
    let source = stated
        .get("source")
        .cloned()
        .ok_or_else(|| refused("the export states no source".to_owned()))?;
    let modules = stated
        .get("modules")
        .cloned()
        .ok_or_else(|| refused("the premises state no modules".to_owned()))?;
    Ok(AcceptedQuirePremises {
        source: serde_json::from_value(source)
            .map_err(|error| refused(format!("source: {error}")))?,
        modules: serde_json::from_value(modules)
            .map_err(|error| refused(format!("modules: {error}")))?,
    })
}

/// One FR-062 result, carried as a value this crate does not interpret.
///
/// The crossing is [`crate::canonical::store_value`]'s — the one
/// `serde_json::Value` ↔ [`JsonValue`] bridge the workspace has — and nothing
/// here orders a member or formats a number.
fn opaque<T: Serialize>(value: &T) -> Result<JsonValue> {
    let value = serde_json::to_value(value).map_err(|error| refused(error.to_string()))?;
    crate::canonical::store_value(&value)
}

/// The refusal the retained `try` catches: an admission that did not happen.
fn refused(message: String) -> GraphAdapterError {
    GraphAdapterError::new(GraphAdapterErrorCode::InvalidPremise, message)
}

/// Which of the three states a load refusal is (`graph-portfolio-load.ts:115-121`).
fn refused_availability<R: GraphInputReader>(
    error: &GraphError,
    reader: &RecordingReader<R>,
    path: &Path,
) -> GapAvailability {
    match *error {
        GraphError::InputInvalid { .. } => GapAvailability::Incompatible,
        // Everything else is `graph-portfolio-load.ts:118-120`'s remaining
        // two arms, which the retained code reaches through one regular
        // expression over the message and this one reaches through the
        // `ErrorKind` the seam reported.
        _ => {
            if reader.was_absent(path) {
                GapAvailability::Missing
            } else {
                GapAvailability::Unreadable
            }
        }
    }
}
