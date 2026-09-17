//! Module: workflow::runtime::cycles::fixture
//!
//! Responsibility: drive the maintained funding attempt and RPC in disposable IC fixtures.
//! Does not own: funding policy, replay storage or an alternative transfer implementation.
//! Boundary: available only with internal-test-fixtures; endpoint fixtures authenticate callers.

use crate::{
    dto::{error::Error, rpc::CyclesResponse},
    model::replay::OperationId,
    ops::{
        ic::IcOps,
        rpc::request::RequestOps,
        storage::async_job_recovery::{AsyncJobCompletion, AsyncJobOwner, AsyncJobRecoveryOps},
    },
    workflow::runtime::async_job::AsyncJobWorkflow,
};

/// Exercise the real durable identity and parent RPC, optionally discarding its reply.
pub async fn request(
    cycles: u128,
    discard_reply: bool,
) -> Result<([u8; 32], Option<CyclesResponse>), Error> {
    let attempt = AsyncJobWorkflow::claim(AsyncJobOwner::CycleTopup)
        .map_err(|_| Error::from_registered(crate::diagnostics::codes::STATE_CONFLICT))?;
    let operation = attempt
        .operation_id(IcOps::canister_self())
        .expect("cycle owner identity");
    let response = RequestOps::request_cycles_with_operation_id(cycles, operation).await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            let outcome = super::CycleWorkflow::finish_parent_funding_failure(
                &cycles.into(),
                operation,
                &error,
            );
            let _ = AsyncJobWorkflow::finish(attempt, outcome);
            return Err(error.into());
        }
    };
    let completion = if discard_reply {
        AsyncJobCompletion::RetryableFailure
    } else {
        AsyncJobCompletion::Success
    };
    assert!(AsyncJobRecoveryOps::finish(
        attempt,
        completion,
        IcOps::now_nanos()
    )?);
    if discard_reply {
        // Deliberately lose the successfully delivered result at the caller's commit boundary.
        // The next invocation must use its persisted pending generation.
        Ok((operation.into_bytes(), None))
    } else {
        Ok((operation.into_bytes(), Some(response)))
    }
}

/// Deliberately submit another actor's identity through the maintained parent transport.
pub async fn collide(cycles: u128, operation_id: [u8; 32]) -> Result<CyclesResponse, Error> {
    RequestOps::request_cycles_with_operation_id(cycles, OperationId::from_bytes(operation_id))
        .await
        .map_err(Into::into)
}
