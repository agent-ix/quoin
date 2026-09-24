// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Agent-IX
//! Caller-controlled cancellation of the currently bounded EA invocation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use engineering_assurance::producer_execution::{CancellationBinding, CancellationToken};

/// Shared control for one sequential campaign run. A request remains pending
/// across member boundaries, while each active EA token keeps its own exact
/// request-authored cancellation event identity.
#[derive(Debug, Default)]
pub struct CampaignCancellation {
    requested: AtomicBool,
    active: Mutex<Option<Arc<CancellationToken>>>,
    active_changed: Condvar,
}

impl CampaignCancellation {
    /// Create an uncancelled control for one run.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cancellation of the active Event-bound EA invocation. A
    /// Disabled binding cannot be cancelled, and is never labeled Cancelled.
    /// The request also applies to later Event-bound invocations in this run.
    pub fn cancel(&self) -> bool {
        self.requested.store(true, Ordering::Release);
        let active = self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        active.as_ref().is_some_and(|token| token.cancel())
    }

    /// Wait for an EA invocation to register its bound token. This is useful
    /// to a controller that must cancel a running process rather than a future
    /// member. The wait is bounded by `timeout`.
    #[must_use]
    pub fn wait_for_active(&self, timeout: Duration) -> bool {
        let active = self
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let (active, _) = self
            .active_changed
            .wait_timeout_while(active, timeout, |token| token.is_none())
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        active.is_some()
    }

    pub(super) fn register(&self, binding: CancellationBinding) -> ActiveCancellation<'_> {
        let token = Arc::new(CancellationToken::new(binding));
        {
            let mut active = self
                .active
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *active = Some(Arc::clone(&token));
            self.active_changed.notify_all();
        }
        if self.requested.load(Ordering::Acquire) {
            token.cancel();
        }
        ActiveCancellation { owner: self, token }
    }
}

pub(super) struct ActiveCancellation<'a> {
    owner: &'a CampaignCancellation,
    token: Arc<CancellationToken>,
}

impl ActiveCancellation<'_> {
    pub(super) fn token(&self) -> &CancellationToken {
        &self.token
    }
}

impl Drop for ActiveCancellation<'_> {
    fn drop(&mut self) {
        let mut active = self
            .owner
            .active
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if active
            .as_ref()
            .is_some_and(|token| Arc::ptr_eq(token, &self.token))
        {
            *active = None;
        }
    }
}
