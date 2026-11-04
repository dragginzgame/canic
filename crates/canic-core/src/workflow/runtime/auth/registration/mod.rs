//! Root registration orchestration; ops owns records, conversion and proof verification.
//! Endpoints authenticate controllers and issuers before entering this workflow.

use crate::{
    model::auth::portable_registration::PortableIssuerPreparationState,
    ops::{
        auth::enrollment::{PortableEnrollmentError, fresh_challenge, registration},
        ic::IcOps,
    },
    workflow::runtime::{auth::RuntimeAuthWorkflow, fleet_activation::FleetActivationWorkflow},
};
use canic_contracts::dto::portable_enrollment::{
    PortableIssuerChallenge, PortableIssuerEnrollment, PortableIssuerRegistrationStatus,
    PreparePortableIssuer, RegisterPortableIssuer,
};
use ic_platform::Principal;

impl RuntimeAuthWorkflow {
    pub(crate) fn retire_portable_issuer(issuer: Principal) -> Result<(), PortableEnrollmentError> {
        require_update()?;
        registration::retire(registration::current_domain()?, issuer, IcOps::now_nanos())
    }

    pub(crate) async fn prepare_portable_issuer(
        request: PreparePortableIssuer,
    ) -> Result<PortableIssuerChallenge, PortableEnrollmentError> {
        require_update()?;
        let domain = registration::current_domain()?;
        match registration::begin(domain.clone(), request, IcOps::now_nanos())? {
            PortableIssuerPreparationState::Ready(challenge) => Ok(challenge),
            PortableIssuerPreparationState::Pending(ticket) => {
                let nonce = fresh_challenge().await?;
                require_update()?;
                if registration::current_domain()? != domain {
                    return Err(PortableEnrollmentError::Binding);
                }
                registration::finish(ticket, nonce, IcOps::now_nanos())
            }
        }
    }

    pub(crate) fn register_portable_issuer(
        caller: Principal,
        request: RegisterPortableIssuer,
    ) -> Result<PortableIssuerEnrollment, PortableEnrollmentError> {
        require_update()?;
        registration::register(
            registration::current_domain()?,
            caller,
            request,
            IcOps::now_nanos(),
        )
    }

    pub(crate) fn portable_issuer_registration(
        caller: Principal,
        operation_id: [u8; 32],
    ) -> Result<PortableIssuerRegistrationStatus, PortableEnrollmentError> {
        FleetActivationWorkflow::require_root_query_allowed("portable_issuer_registration", false)
            .map_err(|_| PortableEnrollmentError::Unavailable)?;
        registration::lookup(
            registration::current_domain()?,
            caller,
            operation_id,
            IcOps::now_nanos(),
        )
    }
}

fn require_update() -> Result<(), PortableEnrollmentError> {
    FleetActivationWorkflow::require_root_update_allowed("portable_issuer_registration", false)
        .map_err(|_| PortableEnrollmentError::Unavailable)
}
