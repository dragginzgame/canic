//! Lossless pool evidence conversions without allocation or settlement policy.
use crate::dto::{
    pool,
    pool_census::{
        Bootstrap, Census, Creation, CreationProgress, Handoff, ImportStatus, Reservation, Source,
        SourceProgress, SourceReceipt,
    },
    pool_import, root,
};
impl From<root::RootPoolReleaseResponse> for Census {
    fn from(value: root::RootPoolReleaseResponse) -> Self {
        Self {
            root: value.root.into(),
            bootstrap: value.bootstrap.map(Into::into),
            capacity_import: value.capacity_import.map(Into::into),
            creation: value.creation.map(Into::into),
            handoff: value.handoff.map(Into::into),
        }
    }
}
impl From<Census> for root::RootPoolReleaseResponse {
    fn from(value: Census) -> Self {
        Self {
            root: value.root.into(),
            bootstrap: value.bootstrap.map(Into::into),
            capacity_import: value.capacity_import.map(Into::into),
            creation: value.creation.map(Into::into),
            handoff: value.handoff.map(Into::into),
        }
    }
}
impl From<root::RootPoolBootstrapReleaseEvidence> for Bootstrap {
    fn from(value: root::RootPoolBootstrapReleaseEvidence) -> Self {
        Self {
            review_sha256: value.review_sha256,
            install_id: value.install_id,
            operator: value.operator.into(),
            store: value.store.into(),
            sources: value.sources.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<Bootstrap> for root::RootPoolBootstrapReleaseEvidence {
    fn from(value: Bootstrap) -> Self {
        Self {
            review_sha256: value.review_sha256,
            install_id: value.install_id,
            operator: value.operator.into(),
            store: value.store.into(),
            sources: value.sources.into_iter().map(Into::into).collect(),
        }
    }
}
impl From<pool_import::PoolImportSource> for Source {
    fn from(value: pool_import::PoolImportSource) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            module_sha256: value.module_sha256,
            canister_version: value.canister_version,
            stopped: value.stopped,
            disposition_sha256: value.disposition_sha256,
            observed_cycles: value.observed_cycles,
            observed_reserved_cycles: value.observed_reserved_cycles,
            minimum_ready_cycles: value.minimum_ready_cycles,
            maximum_debit_cycles: value.maximum_debit_cycles,
        }
    }
}
impl From<Source> for pool_import::PoolImportSource {
    fn from(value: Source) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            controllers: value.controllers.into_iter().map(Into::into).collect(),
            module_sha256: value.module_sha256,
            canister_version: value.canister_version,
            stopped: value.stopped,
            disposition_sha256: value.disposition_sha256,
            observed_cycles: value.observed_cycles,
            observed_reserved_cycles: value.observed_reserved_cycles,
            minimum_ready_cycles: value.minimum_ready_cycles,
            maximum_debit_cycles: value.maximum_debit_cycles,
        }
    }
}
impl From<pool_import::PoolImportReservation> for Reservation {
    fn from(value: pool_import::PoolImportReservation) -> Self {
        Self {
            sequence: value.sequence,
            plan_sha256: value.plan_sha256,
            root_authority_sha256: value.root_authority_sha256,
            root: value.root.into(),
            operator: value.operator.into(),
            subnet: value.subnet.into(),
            transitional_controllers: value
                .transitional_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            final_controllers: value
                .final_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            sources: value.sources.into_iter().map(Into::into).collect(),
            observed_root_cycles: value.observed_root_cycles,
            observed_root_reserved_cycles: value.observed_root_reserved_cycles,
            minimum_root_cycles: value.minimum_root_cycles,
            maximum_root_debit_cycles: value.maximum_root_debit_cycles,
            maximum_paid_calls: value.maximum_paid_calls,
        }
    }
}
impl From<Reservation> for pool_import::PoolImportReservation {
    fn from(value: Reservation) -> Self {
        Self {
            sequence: value.sequence,
            plan_sha256: value.plan_sha256,
            root_authority_sha256: value.root_authority_sha256,
            root: value.root.into(),
            operator: value.operator.into(),
            subnet: value.subnet.into(),
            transitional_controllers: value
                .transitional_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            final_controllers: value
                .final_controllers
                .into_iter()
                .map(Into::into)
                .collect(),
            sources: value.sources.into_iter().map(Into::into).collect(),
            observed_root_cycles: value.observed_root_cycles,
            observed_root_reserved_cycles: value.observed_root_reserved_cycles,
            minimum_root_cycles: value.minimum_root_cycles,
            maximum_root_debit_cycles: value.maximum_root_debit_cycles,
            maximum_paid_calls: value.maximum_paid_calls,
        }
    }
}
impl From<pool_import::PoolImportSourceReceipt> for SourceReceipt {
    fn from(value: pool_import::PoolImportSourceReceipt) -> Self {
        Self {
            root_sender_canister_version: value.root_sender_canister_version,
            canister_id: value.canister_id.into(),
            canister_version: value.canister_version,
            before_uninstall_canister_version: value.before_uninstall_canister_version,
            retained_cycles: value.retained_cycles,
            retained_reserved_cycles: value.retained_reserved_cycles,
            observed_debit_cycles: value.observed_debit_cycles,
        }
    }
}
impl From<SourceReceipt> for pool_import::PoolImportSourceReceipt {
    fn from(value: SourceReceipt) -> Self {
        Self {
            root_sender_canister_version: value.root_sender_canister_version,
            canister_id: value.canister_id.into(),
            canister_version: value.canister_version,
            before_uninstall_canister_version: value.before_uninstall_canister_version,
            retained_cycles: value.retained_cycles,
            retained_reserved_cycles: value.retained_reserved_cycles,
            observed_debit_cycles: value.observed_debit_cycles,
        }
    }
}
impl From<pool_import::PoolImportStatus> for ImportStatus {
    fn from(value: pool_import::PoolImportStatus) -> Self {
        Self {
            reserved_at_ns: value.reserved_at_ns,
            reservation: value.reservation.into(),
            progress: value.progress.into_iter().map(Into::into).collect(),
            phase: value.phase,
            paid_calls: value.paid_calls,
            reserved_debit_cycles: value.reserved_debit_cycles,
            last_root_cycles: value.last_root_cycles,
            root_receipt: value.root_receipt,
        }
    }
}
impl From<ImportStatus> for pool_import::PoolImportStatus {
    fn from(value: ImportStatus) -> Self {
        Self {
            reserved_at_ns: value.reserved_at_ns,
            reservation: value.reservation.into(),
            progress: value.progress.into_iter().map(Into::into).collect(),
            phase: value.phase,
            paid_calls: value.paid_calls,
            reserved_debit_cycles: value.reserved_debit_cycles,
            last_root_cycles: value.last_root_cycles,
            root_receipt: value.root_receipt,
        }
    }
}
impl From<pool::CanisterPoolHandoff> for Handoff {
    fn from(value: pool::CanisterPoolHandoff) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            recipient: value.recipient.into(),
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}
impl From<Handoff> for pool::CanisterPoolHandoff {
    fn from(value: Handoff) -> Self {
        Self {
            canister_id: value.canister_id.into(),
            recipient: value.recipient.into(),
            prepared_at_ns: value.prepared_at_ns,
        }
    }
}
impl From<pool::CanisterPoolCreation> for Creation {
    fn from(value: pool::CanisterPoolCreation) -> Self {
        Self {
            attempt_count: value.attempt_count,
            operation_id: value.operation_id,
            cycles_ledger: value.cycles_ledger.into(),
            placement_subnet: value.placement_subnet.into(),
            root: value.root.into(),
            ledger_amount: value.ledger_amount,
            ledger_fee: value.ledger_fee,
            readiness_floor: value.readiness_floor,
            creation_execution_margin: value.creation_execution_margin,
            management_creation_fee: value.management_creation_fee,
            created_at_time_ns: value.created_at_time_ns,
            last_attempt_at_ns: value.last_attempt_at_ns,
            progress: value.progress.into(),
        }
    }
}
impl From<Creation> for pool::CanisterPoolCreation {
    fn from(value: Creation) -> Self {
        Self {
            attempt_count: value.attempt_count,
            operation_id: value.operation_id,
            cycles_ledger: value.cycles_ledger.into(),
            placement_subnet: value.placement_subnet.into(),
            root: value.root.into(),
            ledger_amount: value.ledger_amount,
            ledger_fee: value.ledger_fee,
            readiness_floor: value.readiness_floor,
            creation_execution_margin: value.creation_execution_margin,
            management_creation_fee: value.management_creation_fee,
            created_at_time_ns: value.created_at_time_ns,
            last_attempt_at_ns: value.last_attempt_at_ns,
            progress: value.progress.into(),
        }
    }
}
impl From<pool_import::PoolImportSourceProgress> for SourceProgress {
    fn from(value: pool_import::PoolImportSourceProgress) -> Self {
        match value {
            pool_import::PoolImportSourceProgress::AwaitingHandoff => Self::AwaitingHandoff,
            pool_import::PoolImportSourceProgress::ControllersIssued => Self::ControllersIssued,
            pool_import::PoolImportSourceProgress::ControllersConfirmed => {
                Self::ControllersConfirmed
            }
            pool_import::PoolImportSourceProgress::StopIssued => Self::StopIssued,
            pool_import::PoolImportSourceProgress::Stopped => Self::Stopped,
            pool_import::PoolImportSourceProgress::UninstallIssued => Self::UninstallIssued,
            pool_import::PoolImportSourceProgress::Ready(receipt) => Self::Ready(receipt.into()),
        }
    }
}
impl From<SourceProgress> for pool_import::PoolImportSourceProgress {
    fn from(value: SourceProgress) -> Self {
        match value {
            SourceProgress::AwaitingHandoff => Self::AwaitingHandoff,
            SourceProgress::ControllersIssued => Self::ControllersIssued,
            SourceProgress::ControllersConfirmed => Self::ControllersConfirmed,
            SourceProgress::StopIssued => Self::StopIssued,
            SourceProgress::Stopped => Self::Stopped,
            SourceProgress::UninstallIssued => Self::UninstallIssued,
            SourceProgress::Ready(receipt) => Self::Ready(receipt.into()),
        }
    }
}
impl From<pool::CanisterPoolCreationProgress> for CreationProgress {
    fn from(value: pool::CanisterPoolCreationProgress) -> Self {
        match value {
            pool::CanisterPoolCreationProgress::Intent { uncertain_result } => {
                Self::Intent { uncertain_result }
            }
            pool::CanisterPoolCreationProgress::Created {
                block_index,
                canister_id,
            } => Self::Created {
                block_index,
                canister_id: canister_id.into(),
            },
            pool::CanisterPoolCreationProgress::WaitingForFunding {
                available,
                attempt_count,
                last_attempt_at_ns,
                observed_at_ns,
                required,
                retry_at_ns,
                shortfall,
            } => Self::WaitingForFunding {
                available,
                attempt_count,
                last_attempt_at_ns,
                observed_at_ns,
                required,
                retry_at_ns,
                shortfall,
            },
            pool::CanisterPoolCreationProgress::Blocked { failure } => Self::Blocked { failure },
        }
    }
}
impl From<CreationProgress> for pool::CanisterPoolCreationProgress {
    fn from(value: CreationProgress) -> Self {
        match value {
            CreationProgress::Intent { uncertain_result } => Self::Intent { uncertain_result },
            CreationProgress::Created {
                block_index,
                canister_id,
            } => Self::Created {
                block_index,
                canister_id: canister_id.into(),
            },
            CreationProgress::WaitingForFunding {
                available,
                attempt_count,
                last_attempt_at_ns,
                observed_at_ns,
                required,
                retry_at_ns,
                shortfall,
            } => Self::WaitingForFunding {
                available,
                attempt_count,
                last_attempt_at_ns,
                observed_at_ns,
                required,
                retry_at_ns,
                shortfall,
            },
            CreationProgress::Blocked { failure } => Self::Blocked { failure },
        }
    }
}
