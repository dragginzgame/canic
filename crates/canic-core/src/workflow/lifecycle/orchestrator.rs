use crate::{
    Error,
    workflow::orchestrator::{CanisterLifecycleOrchestrator, LifecycleEvent, LifecycleResult},
};

/// Apply a lifecycle event via the orchestrator.
///
/// This exists as a named boundary between workflow logic
/// and the lifecycle state machine.
pub async fn apply_event(event: LifecycleEvent) -> Result<LifecycleResult, Error> {
    CanisterLifecycleOrchestrator::apply(event).await
}
