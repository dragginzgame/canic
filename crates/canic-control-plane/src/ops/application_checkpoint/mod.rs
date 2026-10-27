//! Commit original Root checkpoint identity and exact participant acknowledgements.
//!
//! Workflow owns discovery and sequencing; endpoints authenticate callers.

#[cfg(test)]
mod tests;

use crate::{
    storage::stable::application_checkpoint::RootCheckpointStore,
    view::application_checkpoint::RootCheckpointStep,
};
use canic_contracts::{
    dto::application_checkpoint::{
        ApplicationCheckpointCommand, RootCheckpointBeginRequest, RootCheckpointRequest,
        RootCheckpointStatus,
    },
    ids::{
        ApplicationCheckpointBinding, ApplicationCheckpointPhase, CallerReceiverAuthority,
        CallerRootAuthority,
    },
    protocol::application_checkpoint as protocol,
};
use canic_core::control_plane_support::{
    error::InternalError,
    model::application_checkpoint::{RootCheckpointPhase, RootCheckpointRecord},
    ops::{
        application_checkpoint::{authority_digest, command},
        ic::call::CallOps,
    },
};
use sha2::{Digest, Sha256};

/// Stable checkpoint transactions; no method grants snapshot spending authority.
pub struct RootCheckpointOps;

impl RootCheckpointOps {
    pub(crate) fn begin(
        root: &CallerRootAuthority,
        request: &RootCheckpointBeginRequest,
        mut members: Vec<CallerReceiverAuthority>,
    ) -> Result<RootCheckpointStatus, InternalError> {
        if request.root_install_id != root.install_id {
            return Err(InternalError::conflict());
        }
        members.sort_by_key(|member| member.receiver.canister());
        let mut digest = Sha256::new();
        digest.update(b"canic.application-checkpoint.unit.v1\0");
        for member in &members {
            digest.update(authority_digest(member)?);
        }
        let original = RootCheckpointStore::read();
        let next = RootCheckpointRecord::begin(
            original.as_ref(),
            root,
            request.expected_sequence,
            request.operation_id,
            digest.finalize().into(),
            members,
        )
        .map_err(|_| InternalError::conflict())?;
        let status = project(&next);
        if original.as_ref() != Some(&next) {
            RootCheckpointStore::write(next)?;
        }
        Ok(status)
    }

    pub(crate) fn status(
        root: &CallerRootAuthority,
    ) -> Result<Option<RootCheckpointStatus>, InternalError> {
        let Some(record) = RootCheckpointStore::read() else {
            return Ok(None);
        };
        require_root(&record, root)?;
        Ok(Some(project(&record)))
    }

    pub(crate) fn require_catalog_mutation_allowed() -> Result<(), InternalError> {
        if RootCheckpointStore::read().is_some_and(|record| !record.membership_mutation_allowed()) {
            return Err(InternalError::conflict());
        }
        Ok(())
    }

    pub(crate) fn release(
        root: &CallerRootAuthority,
        request: &RootCheckpointRequest,
    ) -> Result<RootCheckpointStatus, InternalError> {
        let original = selected(root, request)?;
        let next = original.release();
        let status = project(&next);
        if next != original {
            RootCheckpointStore::write(next)?;
        }
        Ok(status)
    }

    pub(crate) fn next(
        root: &CallerRootAuthority,
        request: &RootCheckpointRequest,
    ) -> Result<Option<RootCheckpointStep>, InternalError> {
        let record = selected(root, request)?;
        let expected = match record.phase() {
            RootCheckpointPhase::Preparing => ApplicationCheckpointPhase::Prepared,
            RootCheckpointPhase::Releasing => ApplicationCheckpointPhase::Released,
            RootCheckpointPhase::Prepared | RootCheckpointPhase::Released => return Ok(None),
        };
        Ok(record
            .participants()
            .iter()
            .find(|member| member.acknowledged() != Some(expected))
            .map(|member| RootCheckpointStep {
                binding: member.binding().clone(),
                expected,
            }))
    }

    pub(crate) fn acknowledge(
        root: &CallerRootAuthority,
        request: &RootCheckpointRequest,
        binding: &ApplicationCheckpointBinding,
        phase: ApplicationCheckpointPhase,
    ) -> Result<RootCheckpointStatus, InternalError> {
        let original = selected(root, request)?;
        let next = original
            .acknowledge(binding, phase)
            .map_err(|_| InternalError::conflict())?;
        let status = project(&next);
        if next != original {
            RootCheckpointStore::write(next)?;
        }
        Ok(status)
    }
}

fn selected(
    root: &CallerRootAuthority,
    request: &RootCheckpointRequest,
) -> Result<RootCheckpointRecord, InternalError> {
    let record = RootCheckpointStore::read().ok_or_else(InternalError::unavailable)?;
    require_root(&record, root)?;
    let binding = record.participants()[0].binding();
    let expected = RootCheckpointRequest {
        root_install_id: root.install_id,
        operation_id: binding.operation_id,
        sequence: binding.sequence,
    };
    if *request != expected {
        return Err(InternalError::conflict());
    }
    Ok(record)
}

fn require_root(
    record: &RootCheckpointRecord,
    root: &CallerRootAuthority,
) -> Result<(), InternalError> {
    if record.participants()[0].binding().authority.issuer != *root {
        return Err(InternalError::conflict());
    }
    Ok(())
}

fn project(record: &RootCheckpointRecord) -> RootCheckpointStatus {
    let binding = record.participants()[0].binding();
    let (phase, expected) = match record.phase() {
        RootCheckpointPhase::Preparing => (
            ApplicationCheckpointPhase::Preparing,
            ApplicationCheckpointPhase::Prepared,
        ),
        RootCheckpointPhase::Prepared => (
            ApplicationCheckpointPhase::Prepared,
            ApplicationCheckpointPhase::Prepared,
        ),
        RootCheckpointPhase::Releasing => (
            ApplicationCheckpointPhase::Releasing,
            ApplicationCheckpointPhase::Released,
        ),
        RootCheckpointPhase::Released => (
            ApplicationCheckpointPhase::Released,
            ApplicationCheckpointPhase::Released,
        ),
    };
    RootCheckpointStatus {
        root_install_id: binding.authority.issuer.install_id,
        operation_id: binding.operation_id,
        sequence: binding.sequence,
        unit_digest: binding.unit_digest,
        phase,
        participants: u32::try_from(record.participants().len())
            .expect("bounded checkpoint catalogue"),
        acknowledged: u32::try_from(
            record
                .participants()
                .iter()
                .filter(|member| member.acknowledged() == Some(expected))
                .count(),
        )
        .expect("bounded checkpoint catalogue"),
    }
}

/// Send one bounded, replay-safe application hook; acknowledge only its exact identity.
pub async fn invoke(step: &RootCheckpointStep) -> Result<(), canic_contracts::dto::error::Error> {
    let phase = match step.expected {
        ApplicationCheckpointPhase::Prepared => ApplicationCheckpointPhase::Preparing,
        ApplicationCheckpointPhase::Released => ApplicationCheckpointPhase::Releasing,
        _ => return Err(InternalError::invariant().into()),
    };
    let bytes = ic_wire::native::encode_request(
        protocol::PARTICIPANT,
        &command(&step.binding, phase)?,
        protocol::LIMITS,
    )
    .map_err(|_| InternalError::invalid_input())?;
    let reply = CallOps::bounded_wait(
        step.binding.authority.receiver.canister(),
        protocol::PARTICIPANT_METHOD,
    )
    .with_raw_args(bytes)
    .execute()
    .await?
    .into_bytes();
    let observed: Result<ApplicationCheckpointCommand, canic_contracts::dto::error::Error> =
        ic_wire::native::decode_response(protocol::PARTICIPANT, &reply, protocol::LIMITS)
            .map_err(|_| InternalError::invalid_input())?;
    if observed? != command(&step.binding, step.expected)? {
        return Err(InternalError::conflict().into());
    }
    Ok(())
}
