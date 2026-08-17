//! Module: api::component_provisioning
//!
//! Responsibility: marshal root Component provisioning endpoints and transport caller identity.
//! Does not own: authorization policy, validation, persistence, or effects.
//! Boundary: delegates immediately to the root provisioning workflow.

use crate::workflow::component_provisioning;
use canic_core::{
    control_plane_support::ops::ic::IcOps,
    dto::{
        component_provisioning::{
            RootComponentActivationRequest, RootComponentDirectorySynchronizationRequest,
            RootComponentDirectorySynchronizationResponse,
            RootComponentProvisioningAcceptanceRequest, RootComponentProvisioningAdvanceRequest,
            RootComponentProvisioningStatusRequest, RootComponentProvisioningStatusResponse,
            RootComponentPublicationRequest,
        },
        error::Error,
        role::OperationReceipt,
    },
};

/// Root Component Group provisioning endpoint facade.
pub struct RootComponentProvisioningApi;

impl RootComponentProvisioningApi {
    pub async fn accept(
        request: RootComponentProvisioningAcceptanceRequest,
    ) -> Result<OperationReceipt, Error> {
        component_provisioning::accept_and_schedule(IcOps::msg_caller(), request)
            .await
            .map_err(Into::into)
    }

    pub async fn advance(
        request: RootComponentProvisioningAdvanceRequest,
    ) -> Result<RootComponentProvisioningStatusResponse, Error> {
        Box::pin(component_provisioning::advance(
            IcOps::msg_caller(),
            request,
        ))
        .await
        .map_err(Into::into)
    }

    pub fn status(
        request: RootComponentProvisioningStatusRequest,
    ) -> Result<RootComponentProvisioningStatusResponse, Error> {
        component_provisioning::status(IcOps::msg_caller(), request).map_err(Into::into)
    }

    pub async fn publish(
        request: RootComponentPublicationRequest,
    ) -> Result<RootComponentProvisioningStatusResponse, Error> {
        Box::pin(component_provisioning::publish(
            IcOps::msg_caller(),
            request,
        ))
        .await
        .map_err(Into::into)
    }

    pub async fn synchronize_directories(
        request: RootComponentDirectorySynchronizationRequest,
    ) -> Result<RootComponentDirectorySynchronizationResponse, Error> {
        Box::pin(
            crate::workflow::component_directory_synchronization::synchronize(
                IcOps::msg_caller(),
                request,
            ),
        )
        .await
        .map_err(Into::into)
    }

    pub async fn activate(
        request: RootComponentActivationRequest,
    ) -> Result<RootComponentProvisioningStatusResponse, Error> {
        Box::pin(component_provisioning::activate(
            IcOps::msg_caller(),
            request,
        ))
        .await
        .map_err(Into::into)
    }
}
