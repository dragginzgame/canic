//! Module: checkpoint::model
//!
//! Responsibility: freeze exact whole-unit Wire ingress and its original finite plans.
//! Does not own: record publication, fresh observation, transport or remote effect settlement.
//! Boundary: complete retained identities bind requests but never establish current authority.

use crate::backup::checkpoint::CheckpointError;
use canic_contracts::{
    dto::application_checkpoint::{
        RootCheckpointBeginRequest, RootCheckpointRequest, RootCheckpointStatus,
    },
    ids::{ApplicationCheckpointPhase, CallerReceiverAuthority, CallerRootAuthority},
    protocol::application_checkpoint as protocol,
};
use ic_backup::model::{
    application_ingress::ApplicationIngressPayload,
    artifacts::ArtifactChecksumRecord as Digest,
    attempt_journal::AttemptBudgetRecord,
    effect_graph::{EffectGraphRecord, EffectNodeRecord, EffectNodeRequest},
    execution_workflow::ExecutionWorkflowRecord,
    inventory::{
        InventoryRecord, InventoryTargetRecord, InventoryTargetRequest, MAX_INVENTORY_TARGETS,
    },
    operation_plan::{
        OperationPlanRecord, OperationPlanRequest, PlanBudgetRecord, PlanContextRecord,
        PlannedOperationRecord, PlannedOperationRequest,
    },
};
use serde::{Deserialize, Serialize};

pub(super) const RECORD_LIMIT: usize = 1024 * 1024;
const INTENT_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: RECORD_LIMIT,
    allocation: 8 * RECORD_LIMIT,
    nodes: 131_072,
    depth: 64,
};

/// Machine-retained original context, Root installation and complete application unit.
/// This is not observed authority; admission must qualify every member before effects.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CheckpointUnitRecord {
    version: u16,
    context: PlanContextRecord,
    root: CallerRootAuthority,
    members: Vec<CallerReceiverAuthority>,
    operation_id: [u8; 32],
    expected_sequence: u64,
}

impl CheckpointUnitRecord {
    pub fn new(
        context: PlanContextRecord,
        root: CallerRootAuthority,
        members: Vec<CallerReceiverAuthority>,
        operation_id: [u8; 32],
        expected_sequence: u64,
    ) -> Result<Self, CheckpointError> {
        let mut record = Self {
            version: 1,
            context,
            root,
            members,
            operation_id,
            expected_sequence,
        };
        record
            .members
            .sort_by_key(|member| member.receiver.canister());
        record.validate()?;
        Ok(record)
    }
    #[must_use]
    pub fn members(&self) -> &[CallerReceiverAuthority] {
        &self.members
    }
    #[must_use]
    pub const fn root(&self) -> &CallerRootAuthority {
        &self.root
    }
    #[must_use]
    pub const fn context(&self) -> &PlanContextRecord {
        &self.context
    }
    pub(crate) fn validate(&self) -> Result<(), CheckpointError> {
        let correct_members = self.members.iter().all(|member| {
            member.issuer == self.root && member.receiver.canister() != self.root.root
        });
        let canonical = self
            .members
            .windows(2)
            .all(|rows| rows[0].receiver.canister() < rows[1].receiver.canister());
        let bounded_unit = !self.members.is_empty() && self.members.len() < MAX_INVENTORY_TARGETS;
        let valid_operation =
            self.operation_id != [0; 32] && self.expected_sequence.checked_add(1).is_some();
        let same_network = self.context.network()
            == self
                .root
                .registry
                .binding
                .fleet
                .fleet
                .canonical_network_id
                .to_string();
        if !(self.version == 1
            && bounded_unit
            && correct_members
            && canonical
            && valid_operation
            && same_network)
        {
            return Err(CheckpointError::UnitMismatch);
        }
        Ok(())
    }
}

/// One exact original Root update; no signing, dispatch or paid-effect permit.
#[derive(Clone, Debug)]
pub struct CheckpointIngress {
    payload: ApplicationIngressPayload,
    method_id: u64,
    binding: Digest,
    expected: RootCheckpointStatus,
}
impl CheckpointIngress {
    #[must_use]
    pub const fn expected_status(&self) -> &RootCheckpointStatus {
        &self.expected
    }
    #[must_use]
    pub fn receiver(&self) -> &str {
        self.payload.receiver()
    }
    #[must_use]
    pub fn method(&self) -> &str {
        self.payload.method()
    }
    #[must_use]
    pub fn arguments(&self) -> &[u8] {
        self.payload.arguments()
    }
    #[must_use]
    pub const fn binding(&self) -> &Digest {
        &self.binding
    }
    #[must_use]
    pub const fn payload(&self) -> &ApplicationIngressPayload {
        &self.payload
    }
    pub fn decode(&self, bytes: &[u8]) -> Result<RootCheckpointStatus, CheckpointError> {
        let status: Result<RootCheckpointStatus, canic_contracts::dto::error::Error> =
            ic_wire::native::decode_response(self.method_id, bytes, protocol::LIMITS)?;
        let status = status.map_err(|_| CheckpointError::ReplyMismatch)?;
        if status != self.expected {
            return Err(CheckpointError::ReplyMismatch);
        }
        Ok(status)
    }
}

/// Ephemeral exact request catalogue and the single original workflow authority.
#[derive(Clone, Debug)]
pub struct FrozenCheckpoint {
    pub(crate) record: CheckpointUnitRecord,
    workflow: ExecutionWorkflowRecord,
    requests: Vec<CheckpointIngress>,
}
impl FrozenCheckpoint {
    pub fn new(record: CheckpointUnitRecord) -> Result<Self, CheckpointError> {
        record.validate()?;
        ic_wire::native::encode(&record, INTENT_LIMITS)?;
        let unit_digest = protocol::unit_digest(&record.members)?;
        let count =
            u32::try_from(record.members.len()).map_err(|_| CheckpointError::UnitMismatch)?;
        let mut requests = Vec::with_capacity(record.members.len() + 1);
        for acknowledged in 0..=count {
            let (method, method_id, arguments) = if acknowledged == 0 {
                (
                    "canic_root_checkpoint_begin",
                    protocol::BEGIN,
                    ic_wire::native::encode_request(
                        protocol::BEGIN,
                        &RootCheckpointBeginRequest {
                            root_install_id: record.root.install_id,
                            operation_id: record.operation_id,
                            expected_sequence: record.expected_sequence,
                            unit_digest,
                        },
                        protocol::LIMITS,
                    )?,
                )
            } else {
                (
                    "canic_root_checkpoint_advance",
                    protocol::ADVANCE,
                    ic_wire::native::encode_request(
                        protocol::ADVANCE,
                        &RootCheckpointRequest {
                            root_install_id: record.root.install_id,
                            operation_id: record.operation_id,
                            sequence: record.expected_sequence + 1,
                        },
                        protocol::LIMITS,
                    )?,
                )
            };
            let receiver = record.root.root.to_text();
            let payload = ApplicationIngressPayload::new(
                &receiver,
                &receiver,
                method,
                &arguments,
                protocol::LIMITS.bytes,
            )
            .map_err(CheckpointError::accounting)?;
            let binding = payload.digest();
            requests.push(CheckpointIngress {
                payload,
                method_id,
                binding,
                expected: RootCheckpointStatus {
                    root_install_id: record.root.install_id,
                    operation_id: record.operation_id,
                    sequence: record.expected_sequence + 1,
                    unit_digest,
                    phase: if acknowledged == count {
                        ApplicationCheckpointPhase::Prepared
                    } else {
                        ApplicationCheckpointPhase::Preparing
                    },
                    participants: count,
                    acknowledged,
                },
            });
        }
        let plan = build_plan(&record, &requests, None)?;
        Ok(Self {
            record,
            workflow: ExecutionWorkflowRecord::new(plan),
            requests,
        })
    }
    #[must_use]
    pub const fn workflow(&self) -> &ExecutionWorkflowRecord {
        &self.workflow
    }
    pub fn request(&self, sequence: u64) -> Result<&CheckpointIngress, CheckpointError> {
        usize::try_from(sequence)
            .ok()
            .and_then(|index| self.requests.get(index))
            .ok_or(CheckpointError::OriginalMismatch)
    }
    pub(crate) fn child(&self, sequence: u64) -> Result<OperationPlanRecord, CheckpointError> {
        self.request(sequence)?;
        build_plan(&self.record, &self.requests, Some(sequence))
    }
}

fn build_plan(
    record: &CheckpointUnitRecord,
    requests: &[CheckpointIngress],
    only: Option<u64>,
) -> Result<OperationPlanRecord, CheckpointError> {
    let mut targets = vec![record.root.root.to_text()];
    targets.extend(
        record
            .members
            .iter()
            .map(|member| member.receiver.canister().to_text()),
    );
    let inventory = InventoryRecord::new(
        targets
            .into_iter()
            .map(|canister_id| {
                InventoryTargetRecord::new(&InventoryTargetRequest {
                    canister_id,
                    parent_canister_id: None,
                    role: None,
                    module_hash: None,
                })
            })
            .collect::<Result<_, _>>()
            .map_err(CheckpointError::accounting)?,
    )
    .map_err(CheckpointError::accounting)?;
    let selected: Vec<_> = requests
        .iter()
        .enumerate()
        .filter(|(index, _)| only.is_none_or(|only| *index as u64 == only))
        .collect();
    let graph = EffectGraphRecord::new(
        selected
            .iter()
            .map(|(index, _)| {
                EffectNodeRecord::new(EffectNodeRequest {
                    operation_sequence: *index as u64,
                    depends_on: if only.is_none() && *index > 0 {
                        vec![(*index - 1) as u64]
                    } else {
                        vec![]
                    },
                })
            })
            .collect::<Result<_, _>>()
            .map_err(CheckpointError::accounting)?,
    )
    .map_err(CheckpointError::accounting)?;
    let operations = selected
        .iter()
        .map(|(index, request)| {
            PlannedOperationRecord::new(PlannedOperationRequest {
                operation_sequence: *index as u64,
                target: record.root.root.to_text(),
                request: request.binding.hash().into(),
                budget: AttemptBudgetRecord::new(1, 1).map_err(CheckpointError::accounting)?,
            })
            .map_err(CheckpointError::accounting)
        })
        .collect::<Result<Vec<_>, CheckpointError>>()?;
    let count = u32::try_from(operations.len()).map_err(|_| CheckpointError::UnitMismatch)?;
    OperationPlanRecord::new(OperationPlanRequest {
        context: record.context.clone(),
        inventory,
        selected_targets: vec![record.root.root.to_text()],
        graph,
        operations,
        budget: PlanBudgetRecord::new(count, count).map_err(CheckpointError::accounting)?,
    })
    .map_err(CheckpointError::accounting)
}
