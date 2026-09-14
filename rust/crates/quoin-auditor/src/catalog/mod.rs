// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! The verification-method catalog: its shape, its one filesystem seam, and
//! the first-wins merge between them.

pub mod load;
pub mod method;
pub mod source;

pub use load::{load_method_catalog, load_method_catalog_from};
pub use method::{
    DuplicateMethod, MethodCatalog, UnreadableModule, VerificationMethod, method_classes,
};
pub use source::{
    DiskModuleCatalogSource, MemoryModuleCatalogSource, ModuleCatalogSource, ModuleRoot,
};
