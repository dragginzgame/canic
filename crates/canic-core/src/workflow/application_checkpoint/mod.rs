//! Module: workflow::application_checkpoint
//!
//! Responsibility: supply installed receiver authority to the endpoint admission decision.
//! Does not own: application storage/hooks, snapshot effects or Root checkpoint issuance.
//! Boundary: endpoint predicates call this before application transition work.

use crate::{
    domain::policy::pure::application_checkpoint, model::caller_authority::CallerAdmissionError,
    ops::caller_authority::CallerAuthorityOps,
};
use canic_contracts::ids::ApplicationCheckpointBinding;
use ic_platform::principal::ExternalPrincipal as Principal;

pub fn authorize_root(
    requested: &ApplicationCheckpointBinding,
    caller: Principal,
) -> Result<(), CallerAdmissionError> {
    let current =
        CallerAuthorityOps::receiver().ok_or(CallerAdmissionError::AuthorityUnavailable)?;
    application_checkpoint::require_root(requested, &current.authority, caller)
}
