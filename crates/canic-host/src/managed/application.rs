//! Explicit activation uses the original upgrade plan, journal lock and signer owner.
use crate::{
    ApplyOutcome, Error, Result, network::principal, operation, plan, signed, store, supplier,
};
use canic_management_protocol::application::{self, MaintenancePhase};
use std::path::Path;

/// Separately authorize the original Blob attempt after verified closed reopening.
/// No new plan, target, deadline, attempt or installation is admitted here.
/// # Errors
/// Refuses incomplete/changed authority and unresolved original activation requests.
pub async fn activate(
    root: &Path,
    digest: &str,
    identity: &str,
    icp: &Path,
) -> Result<ApplyOutcome> {
    let journal = store::Store::open(root)?;
    let dir = journal.operation(digest)?;
    let (plan, wasm) = plan::load(&dir, digest)?;
    let owner = plan
        .management()
        .filter(|m| m.blob_attempt.is_some())
        .ok_or_else(|| {
            Error::Invalid("activation requires the original Blob upgrade operation".into())
        })?;
    let client = plan.network.client()?;
    let child = plan
        .target()
        .ok_or_else(|| Error::Invalid("activation requires a child upgrade".into()))?;
    crate::auth::permit_upgrade(root, &plan)?;
    let outcome = super::apply(&dir, digest, &plan, &wasm, identity, icp, &client).await?;
    if matches!(outcome, ApplyOutcome::Complete(_)) {
        return Ok(outcome);
    }
    let latest = super::stored::maintenance_evidence(&dir, &plan)?;
    let maintenance = latest.ok_or_else(|| {
        Error::Blocked("retain the original upgrade admission before activation".into())
    })?;
    match maintenance.phase {
        MaintenancePhase::Active => return Ok(outcome),
        MaintenancePhase::ActivateIssued => {
            return super::recovery::reconcile(&dir, digest, &plan, &wasm, identity, icp, &client)
                .await;
        }
        MaintenancePhase::Inspected => {}
        _ => return Err(Error::Blocked(
            "Blob must be reopened and inspected with admission closed before explicit activation"
                .into(),
        )),
    }
    operation::controllers(&client, principal(&owner.root)?, &plan.controller).await?;
    operation::controllers(&client, principal(child)?, &owner.root).await?;
    if operation::module(&client, principal(child)?).await? != plan.artifact.sha256 {
        return Err(Error::Blocked(
            "child code changed before activation".into(),
        ));
    }
    let call = super::Method::ActivateUpgrade.call(&plan, &(), application::LIMITS)?;
    let bytes =
        signed::execute(&dir.join("activate"), &plan, &call, identity, icp, &client).await?;
    let result: std::result::Result<application::Maintenance, canic_management_protocol::Failure> =
        ic_wire::decode(
            &bytes,
            ic_wire::Encoding::Frame {
                method: application::ACTIVATE,
                direction: ic_wire::Direction::Response,
            },
            application::LIMITS,
        )
        .map_err(supplier)?;
    let result = result.map_err(|e| {
        Error::Blocked(format!(
            "activation retained an unresolved outcome: {e:?}; reconcile the original operation"
        ))
    })?;
    if result.phase != MaintenancePhase::Active
        || result.attempt != maintenance.attempt
        || result.arguments != maintenance.arguments
        || result.before != maintenance.before
        || result.after != maintenance.after
        || result.operator != plan.controller
        || result.activator.as_ref() != Some(&plan.controller)
    {
        return Err(Error::Invalid(
            "activation reply changed the approved maintenance custody".into(),
        ));
    }
    super::apply(&dir, digest, &plan, &wasm, identity, icp, &client).await
}
