//! One bounded management observation bracketed by certified physical custody reads.

use crate::{
    canister_protocol::{CanisterProtocolError, inspection::validate_inspection_reserve},
    fleet_ensure::{
        model::capacity_import::{CapacityImportSourceBinding, survey::CapacityImportSampleRecord},
        ops::{
            capacity_import::{admission::declarations::hash, journal::CapacityImportJournalError},
            certified_custody,
        },
        view::certified_custody::CertifiedCanisterCustodyView,
    },
};
use canic_contracts::{
    dto::{canister::CanisterStatusType, error::Error},
    protocol,
};
use ic_platform::{
    agent::Agent,
    management_canister_types::{
        CanisterIdRecord, CanisterStatusResult, CanisterStatusType as ManagementCanisterStatusType,
    },
    principal::ExternalPrincipal as Principal,
};
use std::time::Duration;

/// Free custody checks and encoded arguments retained until the allowance is durable.
pub(in crate::fleet_ensure) struct PreparedManagementObservation {
    agent: Agent,
    canister: Principal,
    before: CertifiedCanisterCustodyView,
    argument: Vec<u8>,
    root: Option<Principal>,
}

impl PreparedManagementObservation {
    /// Expose the free certificate sample before a caller reserves the paid observation.
    pub(in crate::fleet_ensure) const fn custody(&self) -> &CertifiedCanisterCustodyView {
        &self.before
    }

    /// Cross the single paid status boundary and verify custody again afterward.
    pub(in crate::fleet_ensure) async fn observe(
        self,
    ) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
        match self.root {
            Some(root) => observe_root_prepared(self, root).await,
            None => observe_direct_prepared(self).await,
        }
    }
}

/// Inspect a held child through current Root, bracketed by certified custody.
/// Used by callers whose complete inspection round has already been reserved.
pub(in crate::fleet_ensure) async fn observe_root_owned(
    agent: &Agent,
    root: Principal,
    canister: Principal,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    prepare_root_owned(agent, root, canister)
        .await?
        .observe()
        .await
}

/// Check certified custody and encode the Root request without spending an inspection.
pub(in crate::fleet_ensure) async fn prepare_root_owned(
    agent: &Agent,
    root: Principal,
    canister: Principal,
) -> Result<PreparedManagementObservation, CapacityImportJournalError> {
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let before = certified_custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    if !before.controllers.contains(&root) {
        return Err(invalid());
    }
    let target: ic_platform::Principal = canister.into();
    let reserve_argument = ic_wire::native::encode_request(
        protocol::inspection_reserve::METHOD_ID,
        &target,
        protocol::inspection_reserve::REQUEST_LIMITS,
    )
    .map_err(|_| invalid())?;
    let reserve_bytes = tokio::time::timeout(
        Duration::from_secs(30),
        agent
            .query(&root, protocol::inspection_reserve::METHOD)
            .with_arg(reserve_argument)
            .call(),
    )
    .await
    .map_err(|_| invalid())?
    .map_err(|_| invalid())?;
    let reserve: Result<canic_contracts::dto::inspection_reserve::InspectionReserve, Error> =
        ic_wire::native::decode_response(
            protocol::inspection_reserve::METHOD_ID,
            &reserve_bytes,
            protocol::inspection_reserve::RESPONSE_LIMITS,
        )
        .map_err(|_| invalid())?;
    let evidence = reserve.map_err(CapacityImportJournalError::RootRejected)?;
    validate_inspection_reserve(root, canister, evidence.into())?;
    let argument = ic_wire::native::encode_request(
        protocol::inspection::METHOD_ID,
        &canic_contracts::dto::inspection::Request {
            canister_id: canister.into(),
        },
        protocol::inspection::REQUEST_LIMITS,
    )
    .map_err(|_| invalid())?;
    Ok(PreparedManagementObservation {
        agent: agent.clone(),
        canister,
        before,
        argument,
        root: Some(root),
    })
}

async fn observe_root_prepared(
    prepared: PreparedManagementObservation,
    root: Principal,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    let PreparedManagementObservation {
        agent,
        canister,
        before,
        argument,
        ..
    } = prepared;
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let bytes = tokio::time::timeout(
        Duration::from_secs(45),
        agent
            .update(&root, protocol::inspection::METHOD)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| invalid())?
    .map_err(|_| invalid())?;
    let response: Result<canic_contracts::dto::inspection::Response, Error> =
        ic_wire::native::decode_response(
            protocol::inspection::METHOD_ID,
            &bytes,
            protocol::inspection::RESPONSE_LIMITS,
        )
        .map_err(|_| invalid())?;
    let outcome: canic_contracts::dto::canister::CanisterInspectionOutcome = response
        .map_err(CapacityImportJournalError::RootRejected)?
        .try_into()
        .map_err(|_| invalid())?;
    let mut status = match outcome {
        canic_contracts::dto::canister::CanisterInspectionOutcome::Status(status) => status,
        canic_contracts::dto::canister::CanisterInspectionOutcome::ReserveRequired(evidence) => {
            return Err(CanisterProtocolError::inspection_reserve(root, canister, evidence).into());
        }
    };
    status.settings.controllers.sort_unstable();
    let module_sha256 = status
        .module_hash
        .map(|value| value.try_into().map_err(|_| invalid()))
        .transpose()?;
    let after = certified_custody::observe_one(&agent, canister)
        .await
        .map_err(|_| invalid())?;
    let expected = (
        before.subnet,
        &before.controllers,
        before.module_sha256.as_deref().map(hash).transpose()?,
    );
    let actual = (after.subnet, &status.settings.controllers, module_sha256);
    if expected != actual
        || after.controllers != status.settings.controllers
        || after.module_sha256.as_deref().map(hash).transpose()? != module_sha256
        || status.status == CanisterStatusType::Stopping
    {
        return Err(invalid());
    }
    Ok(CapacityImportSampleRecord {
        binding: CapacityImportSourceBinding {
            canister_id: canister,
            subnet: after.subnet,
            controllers: status.settings.controllers,
            module_sha256,
            canister_version: status.version,
            stopped: status.status == CanisterStatusType::Stopped,
            snapshots_size_bytes: status
                .memory_metrics
                .snapshots_size
                .0
                .try_into()
                .map_err(|_| invalid())?,
        },
        cycles: status.cycles.0.try_into().map_err(|_| invalid())?,
        reserved_cycles: status.reserved_cycles.0.try_into().map_err(|_| invalid())?,
    })
}

pub(in crate::fleet_ensure) async fn observe(
    agent: &Agent,
    canister: Principal,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    prepare(agent, canister).await?.observe().await
}

/// Check certified custody and encode the direct request without spending an inspection.
pub(in crate::fleet_ensure) async fn prepare(
    agent: &Agent,
    canister: Principal,
) -> Result<PreparedManagementObservation, CapacityImportJournalError> {
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let before = certified_custody::observe_one(agent, canister)
        .await
        .map_err(|_| invalid())?;
    let argument = ic_platform::candid::contract::encode_one(CanisterIdRecord {
        canister_id: canister,
    })
    .map_err(|_| invalid())?;
    Ok(PreparedManagementObservation {
        agent: agent.clone(),
        canister,
        before,
        argument,
        root: None,
    })
}

async fn observe_direct_prepared(
    prepared: PreparedManagementObservation,
) -> Result<CapacityImportSampleRecord, CapacityImportJournalError> {
    let PreparedManagementObservation {
        agent,
        canister,
        before,
        argument,
        ..
    } = prepared;
    let invalid = || CapacityImportJournalError::ObservationUnavailable { canister };
    let bytes = tokio::time::timeout(
        Duration::from_secs(45),
        agent
            .update(&Principal::management_canister(), "canister_status")
            .with_effective_canister_id(canister)
            .with_arg(argument)
            .call_and_wait(),
    )
    .await
    .map_err(|_| invalid())?
    .map_err(|_| invalid())?;
    let mut status: CanisterStatusResult =
        ic_platform::candid::decoder::decode_one(&bytes).map_err(|_| invalid())?;
    status.settings.controllers.sort_unstable();
    let module_sha256 = status
        .module_hash
        .map(|value| value.try_into().map_err(|_| invalid()))
        .transpose()?;
    let after = certified_custody::observe_one(&agent, canister)
        .await
        .map_err(|_| invalid())?;
    let before_module = before.module_sha256.as_deref().map(hash).transpose()?;
    let after_module = after.module_sha256.as_deref().map(hash).transpose()?;
    let expected = (before.subnet, &before.controllers, before_module);
    let actual = (after.subnet, &status.settings.controllers, module_sha256);
    if expected != actual
        || after.controllers != status.settings.controllers
        || after_module != module_sha256
        || matches!(status.status, ManagementCanisterStatusType::Stopping)
    {
        return Err(invalid());
    }
    Ok(CapacityImportSampleRecord {
        binding: CapacityImportSourceBinding {
            canister_id: canister,
            subnet: after.subnet,
            controllers: status.settings.controllers,
            module_sha256,
            canister_version: status.version,
            stopped: matches!(status.status, ManagementCanisterStatusType::Stopped),
            snapshots_size_bytes: status
                .memory_metrics
                .snapshots_size
                .0
                .try_into()
                .map_err(|_| invalid())?,
        },
        cycles: status.cycles.0.try_into().map_err(|_| invalid())?,
        reserved_cycles: status.reserved_cycles.0.try_into().map_err(|_| invalid())?,
    })
}
