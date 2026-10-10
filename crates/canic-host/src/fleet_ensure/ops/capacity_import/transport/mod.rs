//! Authenticated transport for reviewed host handoff and protected Root import progress.
//!
//! Workflow retains the signed request and issuance before invoking this transport.
//! Management observations alone cannot prove that a pending request has completed.

mod retirement;
mod root;

#[cfg(test)]
pub(crate) mod tests;

use crate::{
    fleet_ensure::{
        model::{
            EffectState,
            capacity_import::{
                CapacityImportHandoffRequestRecord, CapacityImportJournalRecord,
                CapacityImportPlanRecord,
            },
        },
        ops::capacity_import::{
            destination::require_active_reservation, journal, journal::CapacityImportJournalError,
            verify_review,
        },
    },
    icp::{IcpCli, IcpRequestKind},
};
use candid::{CandidType, Principal};
use canic_core::cdk::utils::hash::{decode_hex, hex_bytes};
use ic_agent::{
    Agent,
    agent::{EnvelopeContent, RequestStatusResponse},
};
use serde::Deserialize;
use serde_bytes::ByteBuf;
use sha2_host::{Digest, Sha256};
use std::time::Duration;

pub use retirement::{HandoffOutcome, RetiredHandoff};
pub use root::PreparedRootCommand;

pub(super) const MAXIMUM_ENVELOPE_BYTES: usize = 4_096;
const MAXIMUM_RESPONSE_BYTES: usize = 256 * 1024;
const CALL_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(CandidType)]
struct UpdateControllers {
    canister_id: Principal,
    settings: ControllerSettings,
    sender_canister_version: Option<u64>,
}

#[derive(CandidType)]
struct ControllerSettings {
    controllers: Option<Vec<Principal>>,
}

/// Certified completion bound to the exact approved source and retained request.
/// Only a verified IC response can construct this value; it is not deserializable.
pub struct CompletedHandoff {
    request_id: [u8; 32],
    plan_sha256: [u8; 32],
    canister_id: Principal,
}

impl CompletedHandoff {
    pub(super) fn matches(
        &self,
        plan: &CapacityImportPlanRecord,
        canister_id: Principal,
        request: &CapacityImportHandoffRequestRecord,
    ) -> bool {
        (self.plan_sha256, self.canister_id, self.request_id)
            == (plan.plan_sha256, canister_id, request.request_id)
    }
}

/// Selected authenticated Agent for one operator's reviewed capacity import.
#[derive(Clone)]
pub struct CapacityImportTransport {
    pub(in crate::fleet_ensure::ops::capacity_import) agent: Agent,
    pub(in crate::fleet_ensure::ops::capacity_import) icp: IcpCli,
}

/// A signed handoff whose read-only authority preflight has succeeded.
/// Submission still requires the matching durable Issued journal record.
pub struct PreparedHandoffSubmission {
    agent: Agent,
    icp: IcpCli,
    canister_id: Principal,
    request_id: [u8; 32],
    plan_sha256: [u8; 32],
    bytes: Vec<u8>,
}

impl PreparedHandoffSubmission {
    /// Submit the exact retained ingress after workflow has durably recorded issuance.
    pub async fn submit(
        self,
        journal: &CapacityImportJournalRecord,
    ) -> Result<(), CapacityImportJournalError> {
        let request = issued_request(journal, self.canister_id)?;
        if journal.plan.plan_sha256 != self.plan_sha256 || request.request_id != self.request_id {
            return Err(CapacityImportJournalError::RequestInvalid);
        }
        self.icp
            .measure_async_request(
                IcpRequestKind::AgentUpdate,
                Principal::management_canister(),
                "update_settings",
                Some(self.canister_id),
                async {
                    tokio::time::timeout(
                        CALL_TIMEOUT,
                        self.agent.update_signed(self.canister_id, self.bytes),
                    )
                    .await
                    .map_err(|_| CapacityImportJournalError::Unresolved)?
                    .map_err(|_| CapacityImportJournalError::Unresolved)?;
                    Ok(())
                },
            )
            .await
    }
}

impl CapacityImportTransport {
    /// Resolve the selected ICP signer and network; this performs no handoff.
    pub fn from_icp(icp: &IcpCli) -> Result<Self, CapacityImportJournalError> {
        Ok(Self {
            icp: icp.clone(),
            agent: icp
                .authenticated_agent_with_response_limit(MAXIMUM_RESPONSE_BYTES)
                .map_err(|_| CapacityImportJournalError::ReaderMismatch)?,
        })
    }

    /// Sign locally. The caller must retain this exact envelope before any submission.
    pub fn prepare(
        &self,
        plan: &CapacityImportPlanRecord,
        canister_id: Principal,
    ) -> Result<CapacityImportHandoffRequestRecord, CapacityImportJournalError> {
        verify_agent(&self.agent, plan)?;
        let arg = argument(plan, canister_id)?;
        let signed = self
            .agent
            .update(&Principal::management_canister(), "update_settings")
            .with_effective_canister_id(canister_id)
            .with_arg(arg)
            .expire_after(Duration::from_secs(240))
            .sign()
            .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
        let request = CapacityImportHandoffRequestRecord {
            request_id: *signed.request_id,
            ingress_expiry: signed.ingress_expiry,
            signed_envelope_hex: hex_bytes(signed.signed_update),
        };
        validate_request(plan, canister_id, &request)?;
        Ok(request)
    }

    /// Complete read-only preflight before workflow charges a submission or marks Intent Issued.
    pub async fn prepare_submission(
        &self,
        journal: &CapacityImportJournalRecord,
        canister_id: Principal,
    ) -> Result<PreparedHandoffSubmission, CapacityImportJournalError> {
        journal::validate(journal)?;
        let handoff = journal
            .handoffs
            .iter()
            .find(|handoff| handoff.canister_id == canister_id)
            .ok_or(CapacityImportJournalError::Integrity)?;
        if handoff
            .effect
            .as_ref()
            .is_none_or(|effect| !matches!(effect.state, EffectState::Intent | EffectState::Issued))
        {
            return Err(CapacityImportJournalError::Integrity);
        }
        let request = handoff
            .request
            .as_ref()
            .ok_or(CapacityImportJournalError::Integrity)?;
        verify_agent(&self.agent, &journal.plan)?;
        let bytes = validate_request(&journal.plan, canister_id, request)?;
        let context = self.verify_destination(&journal.plan).await?;
        require_active_reservation(&journal.plan, &context)?;
        Ok(PreparedHandoffSubmission {
            agent: self.agent.clone(),
            icp: self.icp.clone(),
            canister_id,
            request_id: request.request_id,
            plan_sha256: journal.plan.plan_sha256,
            bytes,
        })
    }

    /// Observe the certified reply for the original ingress, including after restart.
    /// Certified terminal status permits custody reconciliation; pending outcomes keep their ingress.
    pub async fn completion(
        &self,
        journal: &CapacityImportJournalRecord,
        canister_id: Principal,
    ) -> Result<HandoffOutcome, CapacityImportJournalError> {
        let request = issued_request(journal, canister_id)?;
        verify_agent(&self.agent, &journal.plan)?;
        self.icp
            .measure_async_request(
                IcpRequestKind::AgentRequestStatus,
                canister_id,
                "read_state",
                Some(canister_id),
                async {
                    let (status, certificate) = tokio::time::timeout(
                        CALL_TIMEOUT,
                        self.agent.request_status_raw(
                            &ic_agent::RequestId::new(&request.request_id),
                            canister_id,
                        ),
                    )
                    .await
                    .map_err(|_| CapacityImportJournalError::Unresolved)?
                    .map_err(|_| CapacityImportJournalError::Unresolved)?;
                    retirement::outcome(&journal.plan, canister_id, request, status, &certificate)
                },
            )
            .await
    }
}

fn issued_request(
    journal: &CapacityImportJournalRecord,
    canister_id: Principal,
) -> Result<&CapacityImportHandoffRequestRecord, CapacityImportJournalError> {
    journal::validate(journal)?;
    let handoff = journal
        .handoffs
        .iter()
        .find(|handoff| handoff.canister_id == canister_id)
        .ok_or(CapacityImportJournalError::Integrity)?;
    if handoff
        .effect
        .as_ref()
        .is_none_or(|effect| effect.state != EffectState::Issued)
    {
        return Err(CapacityImportJournalError::Integrity);
    }
    handoff
        .request
        .as_ref()
        .ok_or(CapacityImportJournalError::Integrity)
}

pub(in crate::fleet_ensure::ops::capacity_import) fn verify_agent(
    agent: &Agent,
    plan: &CapacityImportPlanRecord,
) -> Result<(), CapacityImportJournalError> {
    verify_review(plan, plan.plan_sha256)?;
    let key_hash: [u8; 32] = Sha256::digest(agent.read_root_key()).into();
    let canonical_network = canic_contracts::ids::CanonicalNetworkId::from_der_root_trust_anchor(
        &agent.read_root_key(),
    )
    .map_err(|_| CapacityImportJournalError::ReaderMismatch)?;
    if agent.get_principal().ok() != Some(plan.authority.operator)
        || key_hash != plan.authority.network_root_key_sha256
        || canonical_network != plan.authority.fleet.fleet.canonical_network_id
    {
        return Err(CapacityImportJournalError::ReaderMismatch);
    }
    Ok(())
}

fn argument(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
) -> Result<Vec<u8>, CapacityImportJournalError> {
    if !plan
        .sources
        .iter()
        .any(|source| source.binding.canister_id == canister_id)
    {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    candid::encode_one(UpdateControllers {
        canister_id,
        settings: ControllerSettings {
            controllers: Some(plan.transitional_controllers.clone()),
        },
        sender_canister_version: None,
    })
    .map_err(|_| CapacityImportJournalError::RequestInvalid)
}

pub(super) fn validate_request(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
) -> Result<Vec<u8>, CapacityImportJournalError> {
    if request.signed_envelope_hex.len() > MAXIMUM_ENVELOPE_BYTES * 2 || request.ingress_expiry == 0
    {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    let bytes = decode_hex(&request.signed_envelope_hex)
        .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
    let mut input = bytes.as_slice();
    // Principal's owned-byte visitor is reserved for Candid, so decode IC
    // envelope principals as raw CBOR bytes before constructing the typed call.
    let envelope: EnvelopeWire = ciborium::de::from_reader_with_recursion_limit(&mut input, 32)
        .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
    let content = envelope.content.into_call()?;
    if !input.is_empty() {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    let EnvelopeContent::Call {
        ingress_expiry,
        sender,
        canister_id: target,
        method_name,
        arg,
        ..
    } = &content
    else {
        return Err(CapacityImportJournalError::RequestInvalid);
    };
    let request_id = ic_agent::to_request_id(&content)
        .map_err(|_| CapacityImportJournalError::RequestInvalid)?;
    let expected = (
        request.ingress_expiry,
        plan.authority.operator,
        Principal::management_canister(),
        "update_settings",
        argument(plan, canister_id)?,
    );
    let actual = (
        *ingress_expiry,
        *sender,
        *target,
        method_name.as_str(),
        arg.clone(),
    );
    if actual != expected || *request_id != request.request_id {
        return Err(CapacityImportJournalError::RequestInvalid);
    }
    Ok(bytes)
}

// Called only after Agent authenticated the certificate for this exact request and effective ID.
fn completed(
    plan: &CapacityImportPlanRecord,
    canister_id: Principal,
    request: &CapacityImportHandoffRequestRecord,
    status: RequestStatusResponse,
) -> Result<CompletedHandoff, CapacityImportJournalError> {
    let RequestStatusResponse::Replied(reply) = status else {
        return Err(CapacityImportJournalError::Unresolved);
    };
    if reply.arg
        != candid::encode_args(()).map_err(|_| CapacityImportJournalError::RequestInvalid)?
    {
        return Err(CapacityImportJournalError::Unresolved);
    }
    Ok(CompletedHandoff {
        request_id: request.request_id,
        plan_sha256: plan.plan_sha256,
        canister_id,
    })
}

#[derive(Deserialize)]
struct EnvelopeWire {
    content: CallWire,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CallWire {
    request_type: String,
    nonce: Option<ByteBuf>,
    ingress_expiry: u64,
    sender: ByteBuf,
    canister_id: ByteBuf,
    method_name: String,
    arg: ByteBuf,
    sender_info: Option<serde::de::IgnoredAny>,
}

impl CallWire {
    fn into_call(self) -> Result<EnvelopeContent, CapacityImportJournalError> {
        if self.request_type != "call" || self.sender_info.is_some() {
            return Err(CapacityImportJournalError::RequestInvalid);
        }
        Ok(EnvelopeContent::Call {
            nonce: self.nonce.map(ByteBuf::into_vec),
            ingress_expiry: self.ingress_expiry,
            sender: Principal::try_from_slice(&self.sender)
                .map_err(|_| CapacityImportJournalError::RequestInvalid)?,
            canister_id: Principal::try_from_slice(&self.canister_id)
                .map_err(|_| CapacityImportJournalError::RequestInvalid)?,
            method_name: self.method_name,
            arg: self.arg.into_vec(),
            sender_info: None,
        })
    }
}
