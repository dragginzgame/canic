//! Lossless native projections of original caller-authority receipts.
use crate::{
    dto::{caller_authority as source, caller_authority_status as wire},
    ids,
};
impl From<ids::CallerInstallation> for wire::CallerInstallation {
    fn from(value: ids::CallerInstallation) -> Self {
        Self {
            binding: value.binding.into(),
            install_id: value.install_id,
            component_install_id: value.component_install_id,
        }
    }
}
impl From<wire::CallerInstallation> for ids::CallerInstallation {
    fn from(value: wire::CallerInstallation) -> Self {
        Self {
            binding: value.binding.into(),
            install_id: value.install_id,
            component_install_id: value.component_install_id,
        }
    }
}
impl From<ids::CallerComponentInstallation> for wire::CallerComponentInstallation {
    fn from(value: ids::CallerComponentInstallation) -> Self {
        Self {
            binding: value.binding.into(),
            install_id: value.install_id,
        }
    }
}
impl From<wire::CallerComponentInstallation> for ids::CallerComponentInstallation {
    fn from(value: wire::CallerComponentInstallation) -> Self {
        Self {
            binding: value.binding.into(),
            install_id: value.install_id,
        }
    }
}
impl From<ids::CallerRootAuthority> for wire::CallerRootAuthority {
    fn from(value: ids::CallerRootAuthority) -> Self {
        Self {
            registry: value.registry.into(),
            root: value.root.into(),
            install_id: value.install_id,
        }
    }
}
impl From<wire::CallerRootAuthority> for ids::CallerRootAuthority {
    fn from(value: wire::CallerRootAuthority) -> Self {
        Self {
            registry: value.registry.into(),
            root: value.root.into(),
            install_id: value.install_id,
        }
    }
}
impl From<ids::CallerReceiverAuthority> for wire::CallerReceiverAuthority {
    fn from(value: ids::CallerReceiverAuthority) -> Self {
        Self {
            receiver: value.receiver.into(),
            issuer: value.issuer.into(),
            policy_digest: value.policy_digest,
        }
    }
}
impl From<wire::CallerReceiverAuthority> for ids::CallerReceiverAuthority {
    fn from(value: wire::CallerReceiverAuthority) -> Self {
        Self {
            receiver: value.receiver.into(),
            issuer: value.issuer.into(),
            policy_digest: value.policy_digest,
        }
    }
}
impl From<source::CallerAuthorityPublication> for wire::CallerAuthorityPublication {
    fn from(value: source::CallerAuthorityPublication) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            previous_generation: value.previous_generation,
            generation: value.generation,
            change: value.change.into(),
            content_hash: value.content_hash,
        }
    }
}
impl From<wire::CallerAuthorityPublication> for source::CallerAuthorityPublication {
    fn from(value: wire::CallerAuthorityPublication) -> Self {
        Self {
            operation_id: value.operation_id,
            authority: value.authority.into(),
            previous_generation: value.previous_generation,
            generation: value.generation,
            change: value.change.into(),
            content_hash: value.content_hash,
        }
    }
}
impl From<source::CallerAuthorityReceipt> for wire::CallerAuthorityReceipt {
    fn from(value: source::CallerAuthorityReceipt) -> Self {
        Self {
            publication: value.publication.into(),
            phase: value.phase,
        }
    }
}
impl From<wire::CallerAuthorityReceipt> for source::CallerAuthorityReceipt {
    fn from(value: wire::CallerAuthorityReceipt) -> Self {
        Self {
            publication: value.publication.into(),
            phase: value.phase,
        }
    }
}
impl From<source::CallerAuthorityStatus> for wire::CallerAuthorityStatus {
    fn from(value: source::CallerAuthorityStatus) -> Self {
        Self {
            readiness: value.readiness,
            authority: value.authority.into(),
            generation: value.generation,
            open: value.open,
            retired: value.retired,
            entries: value.entries,
            reserved_bytes: value.reserved_bytes,
            pending_operation: value.pending_operation,
            receipt: value.receipt.map(Into::into),
        }
    }
}
impl From<wire::CallerAuthorityStatus> for source::CallerAuthorityStatus {
    fn from(value: wire::CallerAuthorityStatus) -> Self {
        Self {
            readiness: value.readiness,
            authority: value.authority.into(),
            generation: value.generation,
            open: value.open,
            retired: value.retired,
            entries: value.entries,
            reserved_bytes: value.reserved_bytes,
            pending_operation: value.pending_operation,
            receipt: value.receipt.map(Into::into),
        }
    }
}
impl From<source::CallerAuthorityChange> for wire::CallerAuthorityChange {
    fn from(value: source::CallerAuthorityChange) -> Self {
        match value {
            source::CallerAuthorityChange::StageSource(v) => Self::StageSource(v.into()),
            source::CallerAuthorityChange::Grant(v) => Self::Grant(v.into()),
            source::CallerAuthorityChange::DenySource(v) => Self::DenySource(v.into()),
            source::CallerAuthorityChange::DenyComponent(v) => Self::DenyComponent(v.into()),
            source::CallerAuthorityChange::OpenReceiver => Self::OpenReceiver,
            source::CallerAuthorityChange::RetireReceiver => Self::RetireReceiver,
        }
    }
}
impl From<wire::CallerAuthorityChange> for source::CallerAuthorityChange {
    fn from(value: wire::CallerAuthorityChange) -> Self {
        match value {
            wire::CallerAuthorityChange::StageSource(v) => Self::StageSource(v.into()),
            wire::CallerAuthorityChange::Grant(v) => Self::Grant(v.into()),
            wire::CallerAuthorityChange::DenySource(v) => Self::DenySource(v.into()),
            wire::CallerAuthorityChange::DenyComponent(v) => Self::DenyComponent(v.into()),
            wire::CallerAuthorityChange::OpenReceiver => Self::OpenReceiver,
            wire::CallerAuthorityChange::RetireReceiver => Self::RetireReceiver,
        }
    }
}

/// One selected transition and its native publication; this performs no policy or hashing.
pub struct CallerPublicationRequest {
    pub method: &'static str,
    pub method_id: u64,
    pub publication: wire::CallerAuthorityPublication,
}

/// Select the exact method without changing the original publication.
#[must_use]
pub fn publication_request(command: source::CallerAuthorityCommand) -> CallerPublicationRequest {
    use crate::protocol::caller_authority as protocol;
    let (method, method_id, publication) = match command {
        source::CallerAuthorityCommand::Prepare(value) => {
            (protocol::PREPARE, protocol::PREPARE_ID, value)
        }
        source::CallerAuthorityCommand::Commit(value) => {
            (protocol::COMMIT, protocol::COMMIT_ID, value)
        }
        source::CallerAuthorityCommand::Complete(value) => {
            (protocol::COMPLETE, protocol::COMPLETE_ID, value)
        }
    };
    CallerPublicationRequest {
        method,
        method_id,
        publication: publication.into(),
    }
}
