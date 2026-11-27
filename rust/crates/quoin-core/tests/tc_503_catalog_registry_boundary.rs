// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The core catalog and Quire module resolver retain their canonical owners.

use ix_trace_rs::trace;

const CORE_CATALOG: &str = include_str!("../src/ops/catalog.rs");
const CATALOG_OWNER: &str = include_str!("../../quoin-catalog/src/lib.rs");
const QUIRE_MODULE_RESOLVER: &str = include_str!("../../quoin-quire/src/modules.rs");

#[trace("FR-099-CON-1")]
#[test]
fn catalog_and_module_resolution_delegate_to_their_single_owners() {
    assert!(
        CATALOG_OWNER.contains("pub struct Catalog {"),
        "quoin-catalog owns the assembled catalog type"
    );
    assert!(
        CORE_CATALOG.contains("use quoin_catalog::{Catalog, CatalogError, SemanticView};")
            && CORE_CATALOG.contains("quoin_catalog::build("),
        "the core catalog operation uses the quoin-catalog model and builder"
    );
    assert!(
        !CORE_CATALOG.contains("struct Catalog {") && !CORE_CATALOG.contains("enum Catalog {"),
        "the core operation must not define another catalog model"
    );

    assert!(
        QUIRE_MODULE_RESOLVER.contains("use quire_rs::Registry;")
            && QUIRE_MODULE_RESOLVER.contains("Result<Registry>"),
        "Quoin's module selection returns Quire's registry"
    );
    for constructor in [
        "Registry::load_module_set",
        "Registry::load_module(",
        "Registry::from_env()",
    ] {
        assert!(
            QUIRE_MODULE_RESOLVER.contains(constructor),
            "module resolution delegates to quire-rs through {constructor}"
        );
    }
    assert!(
        !QUIRE_MODULE_RESOLVER.contains("struct Registry {")
            && !QUIRE_MODULE_RESOLVER.contains("enum Registry {"),
        "Quoin's module adapter must not define a replacement registry"
    );
}
