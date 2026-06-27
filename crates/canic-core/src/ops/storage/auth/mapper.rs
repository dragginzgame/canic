//! Module: ops::storage::auth::mapper
//!
//! Responsibility: convert auth DTOs and policies to stable auth records.
//! Does not own: auth verification, storage mutation, or endpoint authorization.
//! Boundary: storage ops conversion layer for persisted auth material.

use crate::{
    domain::policy::auth::{
        RootDelegatedRoleGrantPolicy, RootDelegationAudiencePolicy, RootDelegationRenewalBatch,
        RootIssuerPolicy, RootIssuerRenewalAttempt, RootIssuerRenewalAttemptStatus,
        RootIssuerRenewalOutcome, RootIssuerRenewalProofRef, RootIssuerRenewalState,
        RootIssuerRenewalTemplate,
    },
    dto::auth::{
        ActiveDelegationProof, DelegatedRoleGrant, DelegationAudience, DelegationCert,
        DelegationProof, IcCanisterSignatureProofV1, IssuerProofAlgorithm, IssuerProofBinding,
        RootProof,
    },
    storage::stable::auth::{
        ActiveDelegationProofRecord, DelegatedRoleGrantRecord, DelegationAudienceRecord,
        DelegationCertRecord, DelegationProofRecord, IcCanisterSignatureProofRecord,
        IssuerProofAlgorithmRecord, IssuerProofBindingRecord, RootDelegationRenewalBatchRecord,
        RootIssuerRecord, RootIssuerRenewalAttemptRecord, RootIssuerRenewalAttemptStatusRecord,
        RootIssuerRenewalOutcomeRecord, RootIssuerRenewalProofRefRecord,
        RootIssuerRenewalStateRecord, RootIssuerRenewalTemplateRecord, RootProofRecord,
    },
};

///
/// ActiveDelegationProofRecordMapper
///
/// Storage-ops mapper for active delegation proof DTOs and records.
///

pub struct ActiveDelegationProofRecordMapper;

impl ActiveDelegationProofRecordMapper {
    #[must_use]
    pub fn dto_to_record(proof: ActiveDelegationProof) -> ActiveDelegationProofRecord {
        ActiveDelegationProofRecord {
            proof: delegation_proof_to_record(proof.proof),
            cert_hash: proof.cert_hash,
            not_before_ns: proof.not_before_ns,
            expires_at_ns: proof.expires_at_ns,
            refresh_after_ns: proof.refresh_after_ns,
            installed_at_ns: proof.installed_at_ns,
            installed_by: proof.installed_by,
        }
    }

    #[must_use]
    pub fn record_to_dto(record: ActiveDelegationProofRecord) -> ActiveDelegationProof {
        ActiveDelegationProof {
            proof: delegation_proof_record_to_dto(record.proof),
            cert_hash: record.cert_hash,
            not_before_ns: record.not_before_ns,
            expires_at_ns: record.expires_at_ns,
            refresh_after_ns: record.refresh_after_ns,
            installed_at_ns: record.installed_at_ns,
            installed_by: record.installed_by,
        }
    }
}

///
/// RootIssuerPolicyRecordMapper
///
/// Storage-ops mapper for root issuer policy values and persisted records.
///

pub struct RootIssuerPolicyRecordMapper;

impl RootIssuerPolicyRecordMapper {
    #[must_use]
    pub fn record_to_policy(record: RootIssuerRecord) -> RootIssuerPolicy {
        RootIssuerPolicy {
            issuer_pid: record.issuer_pid,
            enabled: record.enabled,
            allowed_audiences: record
                .allowed_audiences
                .into_iter()
                .map(audience_record_to_policy)
                .collect(),
            allowed_grants: record
                .allowed_grants
                .into_iter()
                .map(grant_record_to_policy)
                .collect(),
            max_cert_ttl_ns: record.max_cert_ttl_ns,
            refresh_after_ratio_bps: record.refresh_after_ratio_bps,
        }
    }

    #[must_use]
    pub fn policy_to_record(policy: RootIssuerPolicy) -> RootIssuerRecord {
        RootIssuerRecord {
            issuer_pid: policy.issuer_pid,
            enabled: policy.enabled,
            allowed_audiences: policy
                .allowed_audiences
                .into_iter()
                .map(audience_policy_to_record)
                .collect(),
            allowed_grants: policy
                .allowed_grants
                .into_iter()
                .map(grant_policy_to_record)
                .collect(),
            max_cert_ttl_ns: policy.max_cert_ttl_ns,
            refresh_after_ratio_bps: policy.refresh_after_ratio_bps,
        }
    }
}

///
/// RootIssuerRenewalTemplateRecordMapper
///
/// Storage-ops mapper for root-managed issuer renewal templates.
///

pub struct RootIssuerRenewalTemplateRecordMapper;

impl RootIssuerRenewalTemplateRecordMapper {
    #[must_use]
    pub fn record_to_template(
        record: RootIssuerRenewalTemplateRecord,
    ) -> RootIssuerRenewalTemplate {
        RootIssuerRenewalTemplate {
            issuer_pid: record.issuer_pid,
            enabled: record.enabled,
            audience: audience_record_to_policy(record.aud),
            grants: record
                .grants
                .into_iter()
                .map(grant_record_to_policy)
                .collect(),
            cert_ttl_ns: record.cert_ttl_ns,
        }
    }

    #[must_use]
    pub fn template_to_record(
        template: RootIssuerRenewalTemplate,
    ) -> RootIssuerRenewalTemplateRecord {
        RootIssuerRenewalTemplateRecord {
            issuer_pid: template.issuer_pid,
            enabled: template.enabled,
            aud: audience_policy_to_record(template.audience),
            grants: template
                .grants
                .into_iter()
                .map(grant_policy_to_record)
                .collect(),
            cert_ttl_ns: template.cert_ttl_ns,
        }
    }
}

///
/// RootIssuerRenewalStateRecordMapper
///
/// Storage-ops mapper for root-managed issuer renewal state.
///

pub struct RootIssuerRenewalStateRecordMapper;

impl RootIssuerRenewalStateRecordMapper {
    #[must_use]
    pub const fn record_to_state(record: RootIssuerRenewalStateRecord) -> RootIssuerRenewalState {
        RootIssuerRenewalState {
            issuer_pid: record.issuer_pid,
            template_fingerprint: record.template_fingerprint,
            last_installed_cert_hash: record.last_installed_cert_hash,
            last_installed_expires_at_ns: record.last_installed_expires_at_ns,
            last_installed_refresh_after_ns: record.last_installed_refresh_after_ns,
            active_attempt_id: record.active_attempt_id,
            last_outcome: renewal_outcome_record_to_policy(record.last_outcome),
            consecutive_failures: record.consecutive_failures,
            next_attempt_after_ns: record.next_attempt_after_ns,
            updated_at_ns: record.updated_at_ns,
        }
    }

    #[must_use]
    pub const fn state_to_record(state: RootIssuerRenewalState) -> RootIssuerRenewalStateRecord {
        RootIssuerRenewalStateRecord {
            issuer_pid: state.issuer_pid,
            template_fingerprint: state.template_fingerprint,
            last_installed_cert_hash: state.last_installed_cert_hash,
            last_installed_expires_at_ns: state.last_installed_expires_at_ns,
            last_installed_refresh_after_ns: state.last_installed_refresh_after_ns,
            active_attempt_id: state.active_attempt_id,
            last_outcome: renewal_outcome_policy_to_record(state.last_outcome),
            consecutive_failures: state.consecutive_failures,
            next_attempt_after_ns: state.next_attempt_after_ns,
            updated_at_ns: state.updated_at_ns,
        }
    }
}

///
/// RootIssuerRenewalAttemptRecordMapper
///
/// Storage-ops mapper for scheduled root-managed issuer renewal attempts.
///

pub struct RootIssuerRenewalAttemptRecordMapper;

impl RootIssuerRenewalAttemptRecordMapper {
    #[must_use]
    pub fn record_to_attempt(record: RootIssuerRenewalAttemptRecord) -> RootIssuerRenewalAttempt {
        RootIssuerRenewalAttempt {
            attempt_id: record.attempt_id,
            issuer_pid: record.issuer_pid,
            template_fingerprint: record.template_fingerprint,
            batch_id: record.batch_id,
            proof_ref: renewal_proof_ref_record_to_policy(record.proof_ref),
            status: renewal_attempt_status_record_to_policy(record.status),
            prepared_at_ns: record.prepared_at_ns,
            retrieval_expires_at_ns: record.retrieval_expires_at_ns,
            install_deadline_ns: record.install_deadline_ns,
            prepared_cert_hash: record.prepared_cert_hash,
            prepared_expires_at_ns: record.prepared_expires_at_ns,
            prepared_refresh_after_ns: record.prepared_refresh_after_ns,
            failure: record.failure.map(renewal_outcome_record_to_policy),
        }
    }

    #[must_use]
    pub fn attempt_to_record(attempt: RootIssuerRenewalAttempt) -> RootIssuerRenewalAttemptRecord {
        RootIssuerRenewalAttemptRecord {
            attempt_id: attempt.attempt_id,
            issuer_pid: attempt.issuer_pid,
            template_fingerprint: attempt.template_fingerprint,
            batch_id: attempt.batch_id,
            proof_ref: renewal_proof_ref_policy_to_record(attempt.proof_ref),
            status: renewal_attempt_status_policy_to_record(attempt.status),
            prepared_at_ns: attempt.prepared_at_ns,
            retrieval_expires_at_ns: attempt.retrieval_expires_at_ns,
            install_deadline_ns: attempt.install_deadline_ns,
            prepared_cert_hash: attempt.prepared_cert_hash,
            prepared_expires_at_ns: attempt.prepared_expires_at_ns,
            prepared_refresh_after_ns: attempt.prepared_refresh_after_ns,
            failure: attempt.failure.map(renewal_outcome_policy_to_record),
        }
    }
}

///
/// RootDelegationRenewalBatchRecordMapper
///
/// Storage-ops mapper for scheduled root-managed renewal batches.
///

pub struct RootDelegationRenewalBatchRecordMapper;

impl RootDelegationRenewalBatchRecordMapper {
    #[must_use]
    pub fn record_to_batch(record: RootDelegationRenewalBatchRecord) -> RootDelegationRenewalBatch {
        RootDelegationRenewalBatch {
            batch_id: record.batch_id,
            attempt_ids: record.attempt_ids,
            prepared_at_ns: record.prepared_at_ns,
            retrieval_expires_at_ns: record.retrieval_expires_at_ns,
        }
    }

    #[must_use]
    pub fn batch_to_record(batch: RootDelegationRenewalBatch) -> RootDelegationRenewalBatchRecord {
        RootDelegationRenewalBatchRecord {
            batch_id: batch.batch_id,
            attempt_ids: batch.attempt_ids,
            prepared_at_ns: batch.prepared_at_ns,
            retrieval_expires_at_ns: batch.retrieval_expires_at_ns,
        }
    }
}

fn delegation_proof_to_record(proof: DelegationProof) -> DelegationProofRecord {
    DelegationProofRecord {
        cert: delegation_cert_to_record(proof.cert),
        root_proof: root_proof_to_record(proof.root_proof),
    }
}

fn delegation_proof_record_to_dto(record: DelegationProofRecord) -> DelegationProof {
    DelegationProof {
        cert: delegation_cert_record_to_dto(record.cert),
        root_proof: root_proof_record_to_dto(record.root_proof),
    }
}

fn delegation_cert_to_record(cert: DelegationCert) -> DelegationCertRecord {
    DelegationCertRecord {
        root_pid: cert.root_pid,
        issuer_pid: cert.issuer_pid,
        issuer_proof_alg: issuer_proof_alg_to_record(cert.issuer_proof_alg),
        issuer_proof_binding_hash: cert.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_proof_binding_to_record(cert.issuer_proof_binding),
        issued_at_ns: cert.issued_at_ns,
        not_before_ns: cert.not_before_ns,
        expires_at_ns: cert.expires_at_ns,
        max_token_ttl_ns: cert.max_token_ttl_ns,
        aud: audience_to_record(cert.aud),
        grants: cert.grants.into_iter().map(grant_to_record).collect(),
    }
}

fn delegation_cert_record_to_dto(record: DelegationCertRecord) -> DelegationCert {
    DelegationCert {
        root_pid: record.root_pid,
        issuer_pid: record.issuer_pid,
        issuer_proof_alg: issuer_proof_alg_record_to_dto(record.issuer_proof_alg),
        issuer_proof_binding_hash: record.issuer_proof_binding_hash,
        issuer_proof_binding: issuer_proof_binding_record_to_dto(record.issuer_proof_binding),
        issued_at_ns: record.issued_at_ns,
        not_before_ns: record.not_before_ns,
        expires_at_ns: record.expires_at_ns,
        max_token_ttl_ns: record.max_token_ttl_ns,
        aud: audience_record_to_dto(record.aud),
        grants: record.grants.into_iter().map(grant_record_to_dto).collect(),
    }
}

fn root_proof_to_record(proof: RootProof) -> RootProofRecord {
    match proof {
        RootProof::IcCanisterSignatureV1(proof) => {
            RootProofRecord::IcCanisterSignatureV1(IcCanisterSignatureProofRecord {
                signature_cbor: proof.signature_cbor,
                public_key_der: proof.public_key_der,
            })
        }
    }
}

fn root_proof_record_to_dto(record: RootProofRecord) -> RootProof {
    match record {
        RootProofRecord::IcCanisterSignatureV1(proof) => {
            RootProof::IcCanisterSignatureV1(IcCanisterSignatureProofV1 {
                signature_cbor: proof.signature_cbor,
                public_key_der: proof.public_key_der,
            })
        }
    }
}

fn audience_to_record(audience: DelegationAudience) -> DelegationAudienceRecord {
    match audience {
        DelegationAudience::Canister(canister) => DelegationAudienceRecord::Canister(canister),
        DelegationAudience::CanicSubnet(subnet) => DelegationAudienceRecord::CanicSubnet(subnet),
        DelegationAudience::Project(project) => DelegationAudienceRecord::Project(project),
    }
}

fn audience_record_to_dto(record: DelegationAudienceRecord) -> DelegationAudience {
    match record {
        DelegationAudienceRecord::Canister(canister) => DelegationAudience::Canister(canister),
        DelegationAudienceRecord::CanicSubnet(subnet) => DelegationAudience::CanicSubnet(subnet),
        DelegationAudienceRecord::Project(project) => DelegationAudience::Project(project),
    }
}

fn grant_to_record(grant: DelegatedRoleGrant) -> DelegatedRoleGrantRecord {
    DelegatedRoleGrantRecord {
        target: grant.target,
        scopes: grant.scopes,
    }
}

fn grant_record_to_dto(record: DelegatedRoleGrantRecord) -> DelegatedRoleGrant {
    DelegatedRoleGrant {
        target: record.target,
        scopes: record.scopes,
    }
}

fn audience_record_to_policy(record: DelegationAudienceRecord) -> RootDelegationAudiencePolicy {
    match record {
        DelegationAudienceRecord::Canister(canister) => {
            RootDelegationAudiencePolicy::Canister(canister)
        }
        DelegationAudienceRecord::CanicSubnet(subnet) => {
            RootDelegationAudiencePolicy::CanicSubnet(subnet)
        }
        DelegationAudienceRecord::Project(project) => {
            RootDelegationAudiencePolicy::Project(project)
        }
    }
}

fn audience_policy_to_record(policy: RootDelegationAudiencePolicy) -> DelegationAudienceRecord {
    match policy {
        RootDelegationAudiencePolicy::Canister(canister) => {
            DelegationAudienceRecord::Canister(canister)
        }
        RootDelegationAudiencePolicy::CanicSubnet(subnet) => {
            DelegationAudienceRecord::CanicSubnet(subnet)
        }
        RootDelegationAudiencePolicy::Project(project) => {
            DelegationAudienceRecord::Project(project)
        }
    }
}

fn grant_record_to_policy(record: DelegatedRoleGrantRecord) -> RootDelegatedRoleGrantPolicy {
    RootDelegatedRoleGrantPolicy {
        target: record.target,
        scopes: record.scopes,
    }
}

fn grant_policy_to_record(policy: RootDelegatedRoleGrantPolicy) -> DelegatedRoleGrantRecord {
    DelegatedRoleGrantRecord {
        target: policy.target,
        scopes: policy.scopes,
    }
}

const fn renewal_outcome_record_to_policy(
    record: RootIssuerRenewalOutcomeRecord,
) -> RootIssuerRenewalOutcome {
    match record {
        RootIssuerRenewalOutcomeRecord::AlreadyInstalled => {
            RootIssuerRenewalOutcome::AlreadyInstalled
        }
        RootIssuerRenewalOutcomeRecord::DriftDetected => RootIssuerRenewalOutcome::DriftDetected,
        RootIssuerRenewalOutcomeRecord::InstallDeadlineExpired => {
            RootIssuerRenewalOutcome::InstallDeadlineExpired
        }
        RootIssuerRenewalOutcomeRecord::Installed => RootIssuerRenewalOutcome::Installed,
        RootIssuerRenewalOutcomeRecord::IssuerCallFailed => {
            RootIssuerRenewalOutcome::IssuerCallFailed
        }
        RootIssuerRenewalOutcomeRecord::NeverRun => RootIssuerRenewalOutcome::NeverRun,
        RootIssuerRenewalOutcomeRecord::PolicyRejected => RootIssuerRenewalOutcome::PolicyRejected,
        RootIssuerRenewalOutcomeRecord::ProofMismatch => RootIssuerRenewalOutcome::ProofMismatch,
        RootIssuerRenewalOutcomeRecord::QuotaExceeded => RootIssuerRenewalOutcome::QuotaExceeded,
        RootIssuerRenewalOutcomeRecord::RejectedByIssuer => {
            RootIssuerRenewalOutcome::RejectedByIssuer
        }
        RootIssuerRenewalOutcomeRecord::RetrievalExpired => {
            RootIssuerRenewalOutcome::RetrievalExpired
        }
        RootIssuerRenewalOutcomeRecord::TemplateChanged => {
            RootIssuerRenewalOutcome::TemplateChanged
        }
        RootIssuerRenewalOutcomeRecord::TemplateDisabled => {
            RootIssuerRenewalOutcome::TemplateDisabled
        }
    }
}

const fn renewal_outcome_policy_to_record(
    outcome: RootIssuerRenewalOutcome,
) -> RootIssuerRenewalOutcomeRecord {
    match outcome {
        RootIssuerRenewalOutcome::AlreadyInstalled => {
            RootIssuerRenewalOutcomeRecord::AlreadyInstalled
        }
        RootIssuerRenewalOutcome::DriftDetected => RootIssuerRenewalOutcomeRecord::DriftDetected,
        RootIssuerRenewalOutcome::InstallDeadlineExpired => {
            RootIssuerRenewalOutcomeRecord::InstallDeadlineExpired
        }
        RootIssuerRenewalOutcome::Installed => RootIssuerRenewalOutcomeRecord::Installed,
        RootIssuerRenewalOutcome::IssuerCallFailed => {
            RootIssuerRenewalOutcomeRecord::IssuerCallFailed
        }
        RootIssuerRenewalOutcome::NeverRun => RootIssuerRenewalOutcomeRecord::NeverRun,
        RootIssuerRenewalOutcome::PolicyRejected => RootIssuerRenewalOutcomeRecord::PolicyRejected,
        RootIssuerRenewalOutcome::ProofMismatch => RootIssuerRenewalOutcomeRecord::ProofMismatch,
        RootIssuerRenewalOutcome::QuotaExceeded => RootIssuerRenewalOutcomeRecord::QuotaExceeded,
        RootIssuerRenewalOutcome::RejectedByIssuer => {
            RootIssuerRenewalOutcomeRecord::RejectedByIssuer
        }
        RootIssuerRenewalOutcome::RetrievalExpired => {
            RootIssuerRenewalOutcomeRecord::RetrievalExpired
        }
        RootIssuerRenewalOutcome::TemplateChanged => {
            RootIssuerRenewalOutcomeRecord::TemplateChanged
        }
        RootIssuerRenewalOutcome::TemplateDisabled => {
            RootIssuerRenewalOutcomeRecord::TemplateDisabled
        }
    }
}

const fn renewal_proof_ref_record_to_policy(
    record: RootIssuerRenewalProofRefRecord,
) -> RootIssuerRenewalProofRef {
    RootIssuerRenewalProofRef {
        issuer_pid: record.issuer_pid,
        cert_hash: record.cert_hash,
    }
}

const fn renewal_proof_ref_policy_to_record(
    proof_ref: RootIssuerRenewalProofRef,
) -> RootIssuerRenewalProofRefRecord {
    RootIssuerRenewalProofRefRecord {
        issuer_pid: proof_ref.issuer_pid,
        cert_hash: proof_ref.cert_hash,
    }
}

const fn renewal_attempt_status_record_to_policy(
    record: RootIssuerRenewalAttemptStatusRecord,
) -> RootIssuerRenewalAttemptStatus {
    match record {
        RootIssuerRenewalAttemptStatusRecord::Prepared => RootIssuerRenewalAttemptStatus::Prepared,
        RootIssuerRenewalAttemptStatusRecord::Installing => {
            RootIssuerRenewalAttemptStatus::Installing
        }
        RootIssuerRenewalAttemptStatusRecord::Installed => {
            RootIssuerRenewalAttemptStatus::Installed
        }
        RootIssuerRenewalAttemptStatusRecord::FailedRetryable => {
            RootIssuerRenewalAttemptStatus::FailedRetryable
        }
        RootIssuerRenewalAttemptStatusRecord::FailedTerminal => {
            RootIssuerRenewalAttemptStatus::FailedTerminal
        }
        RootIssuerRenewalAttemptStatusRecord::Disabled => RootIssuerRenewalAttemptStatus::Disabled,
        RootIssuerRenewalAttemptStatusRecord::Expired => RootIssuerRenewalAttemptStatus::Expired,
    }
}

const fn renewal_attempt_status_policy_to_record(
    status: RootIssuerRenewalAttemptStatus,
) -> RootIssuerRenewalAttemptStatusRecord {
    match status {
        RootIssuerRenewalAttemptStatus::Prepared => RootIssuerRenewalAttemptStatusRecord::Prepared,
        RootIssuerRenewalAttemptStatus::Installing => {
            RootIssuerRenewalAttemptStatusRecord::Installing
        }
        RootIssuerRenewalAttemptStatus::Installed => {
            RootIssuerRenewalAttemptStatusRecord::Installed
        }
        RootIssuerRenewalAttemptStatus::FailedRetryable => {
            RootIssuerRenewalAttemptStatusRecord::FailedRetryable
        }
        RootIssuerRenewalAttemptStatus::FailedTerminal => {
            RootIssuerRenewalAttemptStatusRecord::FailedTerminal
        }
        RootIssuerRenewalAttemptStatus::Disabled => RootIssuerRenewalAttemptStatusRecord::Disabled,
        RootIssuerRenewalAttemptStatus::Expired => RootIssuerRenewalAttemptStatusRecord::Expired,
    }
}

const fn issuer_proof_alg_to_record(alg: IssuerProofAlgorithm) -> IssuerProofAlgorithmRecord {
    match alg {
        IssuerProofAlgorithm::IcCanisterSignatureV1 => {
            IssuerProofAlgorithmRecord::IcCanisterSignatureV1
        }
    }
}

const fn issuer_proof_alg_record_to_dto(
    record: IssuerProofAlgorithmRecord,
) -> IssuerProofAlgorithm {
    match record {
        IssuerProofAlgorithmRecord::IcCanisterSignatureV1 => {
            IssuerProofAlgorithm::IcCanisterSignatureV1
        }
    }
}

const fn issuer_proof_binding_to_record(binding: IssuerProofBinding) -> IssuerProofBindingRecord {
    match binding {
        IssuerProofBinding::IcCanisterSignatureV1 { seed_hash } => {
            IssuerProofBindingRecord::IcCanisterSignatureV1 { seed_hash }
        }
    }
}

const fn issuer_proof_binding_record_to_dto(
    record: IssuerProofBindingRecord,
) -> IssuerProofBinding {
    match record {
        IssuerProofBindingRecord::IcCanisterSignatureV1 { seed_hash } => {
            IssuerProofBinding::IcCanisterSignatureV1 { seed_hash }
        }
    }
}
