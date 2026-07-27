//! Module: model::runtime_kind
//!
//! Responsibility: own the process-local built-in Canic runtime kind.
//! Does not own: persisted role identity, Fleet activation state, or endpoint authorization.
//! Boundary: the dedicated Coordinator init marks its runtime before endpoint dispatch begins.

use std::cell::Cell;

///
/// CanicRuntimeKind
///
/// Process-local distinction between managed Fleet members and the built-in Coordinator.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanicRuntimeKind {
    Managed,
    FleetCoordinator,
}

thread_local! {
    static RUNTIME_KIND: Cell<CanicRuntimeKind> = const { Cell::new(CanicRuntimeKind::Managed) };
}

#[must_use]
pub fn current() -> CanicRuntimeKind {
    RUNTIME_KIND.get()
}

pub fn set(kind: CanicRuntimeKind) {
    RUNTIME_KIND.set(kind);
}
