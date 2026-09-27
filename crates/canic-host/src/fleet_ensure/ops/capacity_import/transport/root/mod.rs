//! Protected Root operations bound to an approved import and completed host handoff.

use crate::fleet_ensure::{
    model::capacity_import::{CapacityImportJournalRecord, CapacityImportPlanRecord},
    ops::capacity_import::{
        journal::{self, CapacityImportJournalError},
        root_reservation,
        transport::{CALL_TIMEOUT, CapacityImportTransport, verify_agent},
        validate_destination_authority, validate_root_status,
    },
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::{
        error::Error,
        fleet_registry::FleetRegistry,
        pool_import::{PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportStatus},
    },
    protocol,
};
use serde::Deserialize;

#[derive(CandidType)]
enum Command {
    ImportPoolCapacity(PoolImportCommand),
}

#[derive(CandidType, Deserialize)]
enum Response {
    ImportPoolCapacity(PoolImportStatus),
}

#[derive(CandidType)]
enum StatusRequest {
    PoolImport(PoolImportIdentity),
    PoolImportContext,
}

#[derive(CandidType, Deserialize)]
enum StatusResponse {
    PoolImport(Box<PoolImportStatus>),
    PoolImportContext(Box<PoolImportContext>),
}

#[derive(CandidType)]
enum CoordinatorRequest {
    Registry,
}

#[derive(CandidType, Deserialize)]
enum CoordinatorResponse {
    Registry(Box<FleetRegistry>),
}

impl CapacityImportTransport {
    /// Verify operator-established Coordinator authority and active Root registration.
    /// Module/provenance observations remain prerequisites owned by the review workflow.
    pub async fn verify_destination(
        &self,
        plan: &CapacityImportPlanRecord,
    ) -> Result<PoolImportContext, CapacityImportJournalError> {
        verify_agent(&self.agent, plan)?;
        let context = self.root_context(plan.authority.root).await?;
        let coordinator = plan.authority.coordinator;
        let argument = candid::encode_one(CoordinatorRequest::Registry)
            .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let bytes = tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent
                .query(&coordinator, protocol::CANIC_COORDINATOR_REGISTRY)
                .with_arg(argument)
                .call(),
        )
        .await
        .map_err(|_| CapacityImportJournalError::CoordinatorUnavailable { coordinator })?
        .map_err(|_| CapacityImportJournalError::CoordinatorUnavailable { coordinator })?;
        let response: Result<CoordinatorResponse, Error> = candid::decode_one(&bytes)
            .map_err(|_| CapacityImportJournalError::CoordinatorUnavailable { coordinator })?;
        let CoordinatorResponse::Registry(registry) =
            response.map_err(CapacityImportJournalError::CoordinatorRejected)?;
        validate_destination_authority(plan, &context, &registry)?;
        crate::fleet_ensure::ops::capacity_import::admission::observer::verify_infrastructure(
            &self.agent,
            plan,
            &registry,
        )
        .await?;
        Ok(context)
    }

    /// Observe protected current authority for review without mutating Root.
    /// The review owner must still bind this to verified Fleet inventory and module authority.
    pub async fn root_context(
        &self,
        root: Principal,
    ) -> Result<PoolImportContext, CapacityImportJournalError> {
        let StatusResponse::PoolImportContext(context) = self
            .root_query(root, StatusRequest::PoolImportContext)
            .await?
        else {
            return Err(CapacityImportJournalError::RootResponseInvalid);
        };
        if context.binding.fleet_subnet_root != root {
            return Err(CapacityImportJournalError::ReaderMismatch);
        }
        Ok(*context)
    }

    /// Recover current progress using the reviewed signer, network and exact Root.
    pub async fn root_status(
        &self,
        plan: &CapacityImportPlanRecord,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        verify_agent(&self.agent, plan)?;
        let StatusResponse::PoolImport(status) = self
            .root_query(
                plan.authority.root,
                StatusRequest::PoolImport(identity(plan)),
            )
            .await?
        else {
            return Err(CapacityImportJournalError::RootResponseInvalid);
        };
        validate_root_status(plan, &status)?;
        Ok(*status)
    }

    /// Establish the exact allocation fence only after durable host approval.
    pub async fn reserve_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        require_approved(journal)?;
        self.root_command(
            &journal.plan,
            PoolImportCommand::Reserve(Box::new(root_reservation(&journal.plan)?)),
        )
        .await
    }

    /// Advance only a source whose original host ingress has certified completion.
    /// Root owns all reset intent, paid-call bounds and lost-response reconciliation.
    pub async fn advance_root(
        &self,
        journal: &CapacityImportJournalRecord,
        canister_id: Principal,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        require_approved(journal)?;
        if !journal::custody_ready(journal, canister_id) {
            return Err(CapacityImportJournalError::Unresolved);
        }
        self.root_command(
            &journal.plan,
            PoolImportCommand::Advance {
                identity: identity(&journal.plan),
                canister_id,
            },
        )
        .await
    }

    /// Request Root's terminal accounting after every retained host handoff has completed.
    pub async fn settle_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        require_approved(journal)?;
        if !journal::all_custody_ready(journal) {
            return Err(CapacityImportJournalError::Unresolved);
        }
        self.root_command(
            &journal.plan,
            PoolImportCommand::Settle(identity(&journal.plan)),
        )
        .await
    }

    /// Release allocation only after recoverable local inventory publication completed.
    pub async fn release_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        require_approved(journal)?;
        if journal
            .operation
            .as_ref()
            .is_none_or(|operation| !operation.publication_complete)
        {
            return Err(CapacityImportJournalError::PublicationConflict);
        }
        let publication_sha256 =
            crate::fleet_ensure::ops::capacity_import::publication::publication_digest(journal)?;
        self.root_command(
            &journal.plan,
            PoolImportCommand::Release {
                identity: identity(&journal.plan),
                publication_sha256,
            },
        )
        .await
    }

    async fn root_command(
        &self,
        plan: &CapacityImportPlanRecord,
        command: PoolImportCommand,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        verify_agent(&self.agent, plan)?;
        self.verify_destination(plan).await?;
        let argument = candid::encode_one(Command::ImportPoolCapacity(command))
            .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let bytes = tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent
                .update(&plan.authority.root, protocol::CANIC_ROOT_COMMAND)
                .with_arg(argument)
                .call_and_wait(),
        )
        .await
        .map_err(|_| CapacityImportJournalError::Unresolved)?
        .map_err(|_| CapacityImportJournalError::Unresolved)?;
        let response: Result<Response, Error> = candid::decode_one(&bytes)
            .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let Response::ImportPoolCapacity(status) =
            response.map_err(CapacityImportJournalError::RootRejected)?;
        validate_root_status(plan, &status)?;
        Ok(status)
    }

    async fn root_query(
        &self,
        root: Principal,
        request: StatusRequest,
    ) -> Result<StatusResponse, CapacityImportJournalError> {
        let argument = candid::encode_one(request)
            .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let bytes = tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent
                .query(&root, protocol::CANIC_ROOT_STATUS)
                .with_arg(argument)
                .call(),
        )
        .await
        .map_err(|_| CapacityImportJournalError::Unresolved)?
        .map_err(|_| CapacityImportJournalError::Unresolved)?;
        let response: Result<StatusResponse, Error> = candid::decode_one(&bytes)
            .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        response.map_err(CapacityImportJournalError::RootRejected)
    }
}

const fn identity(plan: &CapacityImportPlanRecord) -> PoolImportIdentity {
    PoolImportIdentity {
        sequence: plan.authority.import_sequence,
        plan_sha256: plan.plan_sha256,
    }
}

fn require_approved(
    journal: &CapacityImportJournalRecord,
) -> Result<(), CapacityImportJournalError> {
    journal::validate(journal)?;
    if !journal.approved {
        return Err(CapacityImportJournalError::ReservationRequired);
    }
    Ok(())
}
