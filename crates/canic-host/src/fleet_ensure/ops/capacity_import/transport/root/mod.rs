//! Protected Root operations bound to an approved import and completed host handoff.

use crate::{
    fleet_ensure::{
        model::capacity_import::{CapacityImportJournalRecord, CapacityImportPlanRecord},
        ops::capacity_import::{
            journal,
            journal::CapacityImportJournalError,
            root_reservation,
            transport::{CALL_TIMEOUT, CapacityImportTransport, verify_agent},
            validate_destination_authority, validate_root_status,
        },
    },
    icp::IcpRequestKind,
};
use canic_contracts::{
    dto::{
        error::Error,
        pool_import::{PoolImportCommand, PoolImportContext, PoolImportIdentity, PoolImportStatus},
    },
    protocol,
};
use ic_platform::principal::ExternalPrincipal as Principal;

/// One Root update whose local checks and read-only authority preflight succeeded.
/// Workflow persists its submission allowance before consuming this one-use value.

pub struct PreparedRootCommand {
    transport: CapacityImportTransport,
    plan: CapacityImportPlanRecord,
    argument: Vec<u8>,
    method: &'static str,
    method_id: u64,
    subject: Option<Principal>,
}

impl PreparedRootCommand {
    /// Submit without repeating preflight after the durable submission boundary.
    pub async fn submit(self) -> Result<PoolImportStatus, CapacityImportJournalError> {
        self.transport
            .submit_root_command(
                &self.plan,
                self.method,
                self.method_id,
                self.argument,
                self.subject,
            )
            .await
    }
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
        let argument = ic_wire::native::encode_request(
            protocol::registry_discovery::snapshot::METHOD_ID,
            &(),
            protocol::registry_discovery::REQUEST_LIMITS,
        )
        .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let registry = self
            .icp
            .measure_async_request(
                IcpRequestKind::AgentQuery,
                coordinator,
                protocol::registry_discovery::snapshot::METHOD,
                None,
                async {
                    let bytes = tokio::time::timeout(
                        CALL_TIMEOUT,
                        self.agent
                            .query(&coordinator, protocol::registry_discovery::snapshot::METHOD)
                            .with_arg(argument)
                            .call(),
                    )
                    .await
                    .map_err(|_| CapacityImportJournalError::CoordinatorUnavailable {
                        coordinator,
                    })?
                    .map_err(|_| {
                        CapacityImportJournalError::CoordinatorUnavailable { coordinator }
                    })?;
                    let response: Result<
                        canic_contracts::dto::fleet_snapshot::FleetRegistry,
                        Error,
                    > = ic_wire::native::decode_response(
                        protocol::registry_discovery::snapshot::METHOD_ID,
                        &bytes,
                        protocol::registry_discovery::RESPONSE_LIMITS,
                    )
                    .map_err(|_| {
                        CapacityImportJournalError::CoordinatorUnavailable { coordinator }
                    })?;
                    response
                        .map(canic_contracts::dto::fleet_registry::FleetRegistry::from)
                        .map_err(CapacityImportJournalError::CoordinatorRejected)
                },
            )
            .await?;
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
        self.icp
            .measure_async_request(
                IcpRequestKind::AgentQuery,
                root,
                protocol::pool_import_read::context::METHOD,
                None,
                async {
                    let native: canic_contracts::dto::pool_import_read::Context = self
                        .root_query(
                            root,
                            protocol::pool_import_read::context::METHOD,
                            protocol::pool_import_read::context::METHOD_ID,
                            &(),
                        )
                        .await?;
                    let context = PoolImportContext::from(native);
                    if context.binding.fleet_subnet_root != root {
                        return Err(CapacityImportJournalError::ReaderMismatch);
                    }
                    Ok(context)
                },
            )
            .await
    }

    /// Recover current progress using the reviewed signer, network and exact Root.
    pub async fn root_status(
        &self,
        plan: &CapacityImportPlanRecord,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        verify_agent(&self.agent, plan)?;
        self.icp
            .measure_async_request(
                IcpRequestKind::AgentQuery,
                plan.authority.root,
                protocol::pool_import_read::status::METHOD,
                None,
                async {
                    let native: canic_contracts::dto::pool_census::ImportStatus = self
                        .root_query(
                            plan.authority.root,
                            protocol::pool_import_read::status::METHOD,
                            protocol::pool_import_read::status::METHOD_ID,
                            &identity(plan),
                        )
                        .await?;
                    let status = PoolImportStatus::from(native);
                    validate_root_status(plan, &status)?;
                    Ok(status)
                },
            )
            .await
    }

    /// Establish the exact allocation fence only after durable host approval.
    pub async fn prepare_reserve_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PreparedRootCommand, CapacityImportJournalError> {
        require_approved(journal)?;
        self.prepare_root_command(
            &journal.plan,
            PoolImportCommand::Reserve(Box::new(root_reservation(&journal.plan)?)),
        )
        .await
    }

    /// Advance only a source whose original host ingress has certified completion.
    /// Root owns all reset intent, paid-call bounds and lost-response reconciliation.
    pub async fn prepare_advance_root(
        &self,
        journal: &CapacityImportJournalRecord,
        canister_id: Principal,
    ) -> Result<PreparedRootCommand, CapacityImportJournalError> {
        require_approved(journal)?;
        if !journal::custody_ready(journal, canister_id) {
            return Err(CapacityImportJournalError::Unresolved);
        }
        self.prepare_root_command(
            &journal.plan,
            PoolImportCommand::Advance {
                identity: identity(&journal.plan),
                canister_id,
            },
        )
        .await
    }

    /// Request Root's terminal accounting after every retained host handoff has completed.
    pub async fn prepare_settle_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PreparedRootCommand, CapacityImportJournalError> {
        require_approved(journal)?;
        if !journal::all_custody_ready(journal) {
            return Err(CapacityImportJournalError::Unresolved);
        }
        self.prepare_root_command(
            &journal.plan,
            PoolImportCommand::Settle(identity(&journal.plan)),
        )
        .await
    }

    /// Release allocation only after recoverable local inventory publication completed.
    pub async fn prepare_release_root(
        &self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<PreparedRootCommand, CapacityImportJournalError> {
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
        self.prepare_root_command(
            &journal.plan,
            PoolImportCommand::Release {
                identity: identity(&journal.plan),
                publication_sha256,
            },
        )
        .await
    }

    async fn prepare_root_command(
        &self,
        plan: &CapacityImportPlanRecord,
        command: PoolImportCommand,
    ) -> Result<PreparedRootCommand, CapacityImportJournalError> {
        use canic_contracts::{
            dto::{import_control, pool_census},
            protocol::import_control as wire,
        };
        verify_agent(&self.agent, plan)?;
        self.verify_destination(plan).await?;
        let subject = match &command {
            PoolImportCommand::Advance { canister_id, .. } => Some(*canister_id),
            _ => None,
        };
        let (method, method_id, argument) = match command {
            PoolImportCommand::Reserve(reservation) => (
                wire::reserve::METHOD,
                wire::reserve::METHOD_ID,
                ic_wire::native::encode_request(
                    wire::reserve::METHOD_ID,
                    &pool_census::Reservation::from(*reservation),
                    wire::REQUEST_LIMITS,
                ),
            ),
            PoolImportCommand::Advance {
                identity,
                canister_id,
            } => (
                wire::advance::METHOD,
                wire::advance::METHOD_ID,
                ic_wire::native::encode_request(
                    wire::advance::METHOD_ID,
                    &import_control::AdvanceRequest {
                        identity,
                        canister_id: canister_id.into(),
                    },
                    wire::REQUEST_LIMITS,
                ),
            ),
            PoolImportCommand::Settle(identity) => (
                wire::settle::METHOD,
                wire::settle::METHOD_ID,
                ic_wire::native::encode_request(
                    wire::settle::METHOD_ID,
                    &identity,
                    wire::REQUEST_LIMITS,
                ),
            ),
            PoolImportCommand::Release {
                identity,
                publication_sha256,
            } => (
                wire::release::METHOD,
                wire::release::METHOD_ID,
                ic_wire::native::encode_request(
                    wire::release::METHOD_ID,
                    &import_control::ReleaseRequest {
                        identity,
                        publication_sha256,
                    },
                    wire::REQUEST_LIMITS,
                ),
            ),
        };
        let argument = argument.map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        Ok(PreparedRootCommand {
            transport: self.clone(),
            plan: plan.clone(),
            argument,
            method,
            method_id,
            subject,
        })
    }

    async fn submit_root_command(
        &self,
        plan: &CapacityImportPlanRecord,
        method: &'static str,
        method_id: u64,
        argument: Vec<u8>,
        subject: Option<Principal>,
    ) -> Result<PoolImportStatus, CapacityImportJournalError> {
        let result = self
            .icp
            .measure_async_request(
                IcpRequestKind::AgentUpdate,
                plan.authority.root,
                method,
                subject,
                async {
                    let bytes = tokio::time::timeout(
                        CALL_TIMEOUT,
                        self.agent
                            .update(&plan.authority.root, method)
                            .with_arg(argument)
                            .call_and_wait(),
                    )
                    .await
                    .map_err(|_| CapacityImportJournalError::Unresolved)?
                    .map_err(|_| CapacityImportJournalError::Unresolved)?;
                    let response: Result<canic_contracts::dto::pool_census::ImportStatus, Error> =
                        ic_wire::native::decode_response(
                            method_id,
                            &bytes,
                            protocol::import_control::RESPONSE_LIMITS,
                        )
                        .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
                    let status = PoolImportStatus::from(
                        response.map_err(CapacityImportJournalError::RootRejected)?,
                    );
                    validate_root_status(plan, &status)?;
                    Ok(status)
                },
            )
            .await;
        match result {
            Err(CapacityImportJournalError::RootRejected(error)) => {
                if error.code() == canic_contracts::diagnostics::codes::CAPACITY_LIMIT.raw_code()
                    && let Ok(status) = self.root_status(plan).await
                {
                    return Err(CapacityImportJournalError::RootCapacityLimit {
                        phase: status.phase,
                        reserved_debit_cycles: status.reserved_debit_cycles,
                        maximum_debit_cycles: status.reservation.maximum_root_debit_cycles,
                        observed_debit_cycles: status
                            .reservation
                            .observed_root_cycles
                            .saturating_sub(status.last_root_cycles),
                        paid_calls: status.paid_calls,
                        maximum_paid_calls: status.reservation.maximum_paid_calls,
                    });
                }
                Err(CapacityImportJournalError::RootRejected(error))
            }
            result => result,
        }
    }

    async fn root_query<I: serde::Serialize + Sync, O: serde::de::DeserializeOwned>(
        &self,
        root: Principal,
        method: &'static str,
        method_id: u64,
        request: &I,
    ) -> Result<O, CapacityImportJournalError> {
        let argument = ic_wire::native::encode_request(
            method_id,
            request,
            protocol::pool_import_read::REQUEST_LIMITS,
        )
        .map_err(|_| CapacityImportJournalError::RootResponseInvalid)?;
        let bytes = tokio::time::timeout(
            CALL_TIMEOUT,
            self.agent.query(&root, method).with_arg(argument).call(),
        )
        .await
        .map_err(|_| CapacityImportJournalError::Unresolved)?
        .map_err(|_| CapacityImportJournalError::Unresolved)?;
        let response: Result<O, Error> = ic_wire::native::decode_response(
            method_id,
            &bytes,
            protocol::pool_import_read::RESPONSE_LIMITS,
        )
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
