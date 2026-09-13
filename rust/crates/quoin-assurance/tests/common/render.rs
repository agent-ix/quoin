// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The build-case-to-rendered-markdown hop the FR-040 rendering and
//! assessment-section suites share.

use quoin_assurance::case::AssuranceCase;
use quoin_assurance::render::{RenderableCase, render_case};

/// The built case as the renderer reads it.
///
/// `build_case` carries both assessment lists as opaque values; `render_case`
/// reads fourteen of their fields. The hop between the two type parameters is
/// the payload itself, which is how `quoin-core` does it — so this is the real
/// path a caller takes, not a test-only shortcut around the seam.
fn renderable(case: &AssuranceCase) -> RenderableCase {
    let payload = serde_json::to_value(case).expect("the case serialises");
    serde_json::from_value(payload).expect("the payload is a renderable case")
}

pub(crate) fn render(case: &AssuranceCase) -> String {
    render_case(&renderable(case))
}
