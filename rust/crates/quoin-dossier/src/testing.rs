// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! Fixtures the unit tests of this crate share: a configurable generator, a
//! registry builder, run inputs and a directory listing.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "in a test, a panic IS the failure report; the production lints stand"
)]

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::SourceFailure;
use crate::gap::Gap;
use crate::registry::{DocumentSpec, Generated, Generator, Inputs, Registry};

/// A generator whose gaps and failure a test sets.
pub(crate) struct Doc {
    pub(crate) spec: DocumentSpec,
    pub(crate) gaps: fn() -> Vec<Gap>,
    pub(crate) fail: bool,
}

impl Generator for Doc {
    fn spec(&self) -> DocumentSpec {
        self.spec
    }

    fn generate(&self, _: &Inputs) -> Result<Generated, SourceFailure> {
        if self.fail {
            return Err(SourceFailure {
                source: "requirements".to_owned(),
                detail: "unreadable".to_owned(),
            });
        }
        Ok(Generated {
            body: format!("# {}", self.spec.id),
            document_version: None,
            gaps: (self.gaps)(),
        })
    }
}

/// A complete document with no gaps that runs cleanly.
pub(crate) fn doc(kind: &'static str, id: &'static str, needs_as_of: bool) -> Doc {
    Doc {
        spec: DocumentSpec {
            kind,
            id,
            title: "T",
            needs_as_of,
        },
        gaps: Vec::new,
        fail: false,
    }
}

/// Run inputs at version `1.0`, with no `--as-of`.
pub(crate) fn inputs() -> Inputs {
    Inputs {
        scope: PathBuf::from("."),
        modules: Vec::new(),
        clause_bindings: Vec::new(),
        pack: None,
        soup_binding: None,
        soup_records: None,
        soup_support: None,
        snapshot: None,
        release_verify: None,
        as_of: None,
        version: "1.0".to_owned(),
    }
}

/// A registry holding `docs` in order.
pub(crate) fn registry(docs: Vec<Doc>) -> Registry {
    let mut registry = Registry::new();
    for doc in docs {
        registry.register(Box::new(doc)).unwrap();
    }
    registry
}

/// The sorted file names in `dir`.
pub(crate) fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// Sources that must not be reached by registry and unrelated document tests.
pub(crate) fn sources() -> crate::Sources {
    crate::Sources {
        matrix: std::sync::Arc::new(NoMatrix),
        rmm: |_| panic!("unexpected discharge source"),
        soup: |_| panic!("unexpected SOUP source"),
        policy: |_| panic!("unexpected policy source"),
        threat_model: |_| panic!("unexpected threat-model source"),
        records_of_processing: |_| panic!("unexpected records-of-processing source"),
    }
}

struct NoMatrix;

impl crate::MatrixSource for NoMatrix {
    fn matrix(
        &self,
        _inputs: &Inputs,
    ) -> Result<quoin_assurance::matrix::MatrixOutput, SourceFailure> {
        Err(SourceFailure {
            source: "matrix".to_owned(),
            detail: "not used by these tests".to_owned(),
        })
    }
}
