//! Canic admission plus the application-owner boundary; handlers delegate immediately.
use super::{
    dto::{EnqueueRequest, ReconcileRequest},
    model, protocol,
    view::{DeliveryEvidence, JobsView},
    workflow,
};
use canic::{Error, prelude::*};
fn authorize() -> Result<(), Error> {
    let caller = canic::fleet_admission::require_caller()?;
    let owner = ic_platform::principal::ExternalPrincipal::from_slice(&[17; 29]);
    if caller != owner {
        return Err(Error::from_registered(
            canic::diagnostics::codes::AUTHORITY_UNAUTHORIZED,
        ));
    }
    Ok(())
}
#[update(public, wire(id = protocol::ENQUEUE, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_enqueue(request: EnqueueRequest) -> Result<u128, Error> {
    authorize()?;
    workflow::enqueue(request.delta, request.delay_secs)
}
#[update(public, wire(id = protocol::DRAIN, request = protocol::LIMITS, response = protocol::LIMITS))]
async fn managed_jobs_drain() -> Result<(), Error> {
    authorize()?;
    workflow::drain().await
}
#[query(public, wire(id = protocol::STATUS, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_status() -> Result<JobsView, Error> {
    authorize()?;
    workflow::view()
}
#[update(public, wire(id = protocol::EFFECT, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_effect(evidence: DeliveryEvidence) -> Result<DeliveryEvidence, Error> {
    if ic_platform::cdk::api::msg_caller() != ic_platform::cdk::api::canister_self() {
        return Err(model::conflict());
    }
    workflow::receive(evidence)
}
#[update(public, wire(id = protocol::RECONCILE, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_reconcile(request: ReconcileRequest) -> Result<(), Error> {
    authorize()?;
    workflow::reconcile(request.job, request.sequence)
}
#[update(public, wire(id = protocol::FAULT, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_fault(mode: u8) -> Result<(), Error> {
    authorize()?;
    workflow::set_fault(mode)
}
#[update(public, wire(id = protocol::CORRUPT, request = protocol::LIMITS, response = protocol::LIMITS))]
fn managed_jobs_corrupt() -> Result<(), Error> {
    authorize()?;
    workflow::corrupt_for_qualification();
    Ok(())
}
