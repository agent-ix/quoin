// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX

//! ADR-0011 invariant 1, enforced by the type system.
//!
//! > **The auditor runs nothing.** It reads the store and reports
//! > (`src/auditor/audit.ts:14`); the consumer's CI refreshes. That separation
//! > is what lets the report be trusted: an auditor that could re-run a suite
//! > could also make a finding disappear by re-running it.
//!
//! # "We didn't call anything" is not an enforcement
//!
//! The retained TypeScript states the invariant in a comment and keeps it by
//! habit. A comment is not a check, and `audit()` there could grow a
//! `child_process` import in one line with nothing to stop it.
//!
//! This crate makes execution **unrepresentable** instead, and the mechanism is
//! [`Inert`]: a sealed marker whose supertrait is [`serde::de::DeserializeOwned`].
//!
//! # Why `DeserializeOwned` is the proof and not a proxy for one
//!
//! A value the auditor may be handed must be reconstructible **from bytes
//! alone**. That single requirement rules out every way of naming the world:
//!
//! | to run something you need | why it cannot be in an `Inert` type |
//! |---|---|
//! | `std::process::Command` | not [`Deserialize`](serde::Deserialize) |
//! | a file descriptor, `File`, `TcpStream` | not [`Deserialize`](serde::Deserialize) |
//! | a closure (`impl Fn`, `fn` pointer) | not [`Deserialize`](serde::Deserialize) |
//! | a trait object (`Box<dyn Trait>`) | not [`Sized`], not [`Deserialize`](serde::Deserialize) |
//! | a handle borrowed from the caller | `DeserializeOwned` forbids the borrow |
//!
//! This is checked by `rustc`, not by a reviewer: `#[derive(Deserialize)]` on
//! [`AuditInput`](crate::AuditInput) fails to compile the moment a field can
//! hold any of the above, and the [`Inert`] impl fails with it.
//!
//! Crucially, the crate's **one** world-touching seam —
//! [`ModuleCatalogSource`](crate::catalog::ModuleCatalogSource) — is a trait.
//! `dyn ModuleCatalogSource` is not `DeserializeOwned`, so it cannot appear
//! anywhere inside an `Inert` type. The auditor mechanically cannot name the
//! thing that touches a filesystem, let alone one that starts a process.
//!
//! # What this does not claim
//!
//! It does not claim the crate contains no I/O: [`catalog`](crate::catalog)
//! reads `manifest.yaml`, on purpose, behind that one trait. It claims that
//! the *audit* and *advise* surfaces cannot reach it. `tests/tc_383_inert.rs`
//! states the second half as a source census over `src/audit/` and
//! `src/advise/`, with an anti-vacuity floor, because a type bound says
//! nothing about a function that ignores its arguments and shells out anyway.

mod sealed {
    /// Sealed so that [`Inert`](super::Inert) cannot be claimed from outside.
    ///
    /// Without this, a downstream crate could implement `Inert` for a type
    /// holding a `Box<dyn FnMut()>` behind a hand-written `Deserialize` that
    /// panics, and the marker would stop meaning anything.
    pub trait Sealed {}
}

/// A value that carries data and nothing else.
///
/// See the module documentation: the supertrait is the enforcement.
pub trait Inert: serde::de::DeserializeOwned + Sized + sealed::Sealed {}

macro_rules! inert {
    ($($type:ty),* $(,)?) => {
        $(
            impl sealed::Sealed for $type {}
            impl Inert for $type {}
        )*
    };
}

inert!(
    crate::audit::AuditInput,
    crate::advise::ObligationFacts,
    crate::catalog::MethodCatalog,
);

/// Compile-time assertion that every input surface is [`Inert`].
///
/// A `const` item, so it is checked whether or not tests are built. If someone
/// adds a field that can hold a handle, this stops compiling.
const _: fn() = || {
    const fn only_inert<T: Inert>() {}
    only_inert::<crate::audit::AuditInput>();
    only_inert::<crate::advise::ObligationFacts>();
    only_inert::<crate::catalog::MethodCatalog>();
};
