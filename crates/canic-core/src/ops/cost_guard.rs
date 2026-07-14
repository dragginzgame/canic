//! Module: ops::cost_guard
//!
//! Responsibility: reserve and settle mechanical cost guard quota/cycle permits.
//! Does not own: workflow authorization, command semantics, or public error mapping.
//! Boundary: workflows submit cost guard requests before external-effect boundaries.

use crate::{
    InternalError, InternalErrorOrigin,
    cdk::types::Principal,
    ids::{IntentId, IntentResourceKey},
    model::replay::CommandKind,
    ops::storage::intent::IntentStoreOps,
    replay_policy::CostClass,
};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use thiserror::Error as ThisError;

const KEY_HASH_BYTES: usize = 8;
const MIN_QUOTA_WINDOW_SECONDS: u64 = 1;
const INTENT_TTL_SECONDS: u64 = 60 * 60;

///
/// CostGuardPermit
///
/// Durable permit proving a new costed operation reserved quota and cycle
/// budget before crossing an expensive external-effect boundary.
///

#[derive(Debug)]
pub struct CostGuardPermit {
    _cost_class: CostClass,
    _quota_key: IntentResourceKey,
    pub reservation_id: IntentId,
    _payer: Principal,
    quota_intent_id: IntentId,
    _private: (),
}

///
/// CostGuardRequest
///
/// Input for reserving quota and cycle budget for a costed command.
/// Owned by ops and supplied by workflows before external effects.
///

#[derive(Clone, Debug)]
pub struct CostGuardRequest {
    pub cost_class: CostClass,
    pub command_kind: CommandKind,
    pub quota_subject: Principal,
    pub payer: Principal,
    pub now_secs: u64,
    pub quota_window_secs: u64,
    pub max_operations_per_window: u64,
    pub current_cycle_balance: u128,
    pub cycle_reservation_cycles: u128,
    pub min_cycles_after_reservation: u128,
}

///
/// CostGuardReservePublicKind
///
/// Public-facing class for cost guard reservation failures.
/// Owned by ops and mapped by workflow into boundary errors.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CostGuardReservePublicKind {
    InvalidInput,
    ResourceExhausted,
}

///
/// CostGuardReserveError
///
/// Typed cost guard reservation failure.
/// Owned by ops and returned before a permit is issued.
///

#[derive(Debug, ThisError)]
pub enum CostGuardReserveError {
    #[error("cost guard requires a costed operation class")]
    UncostedClass,

    #[error("cost guard quota window must be greater than zero")]
    InvalidQuotaWindow,

    #[error("cost guard quota rejects all new operations")]
    QuotaRejectsAll,

    #[error("cost guard cycle reservation exceeds supported accounting range")]
    CycleReservationOverflow,

    #[error("cost guard quota exceeded for {command_kind}: used={used} max={max}")]
    QuotaExceeded {
        command_kind: String,
        used: u64,
        max: u64,
    },

    #[error(
        "cost guard cycle reserve rejected {command_kind}: available={available} required={required}"
    )]
    CycleReserveRejected {
        command_kind: String,
        available: u128,
        required: u128,
    },

    #[error("cost guard resource key is invalid: {0}")]
    ResourceKeyInvalid(String),

    #[error(transparent)]
    Store(#[from] InternalError),
}

impl CostGuardReserveError {
    /// public_kind
    ///
    /// Return the boundary-safe class for reservation failures that callers may expose.
    #[must_use]
    pub const fn public_kind(&self) -> Option<CostGuardReservePublicKind> {
        match self {
            Self::UncostedClass | Self::InvalidQuotaWindow | Self::ResourceKeyInvalid(_) => {
                Some(CostGuardReservePublicKind::InvalidInput)
            }
            Self::QuotaRejectsAll
            | Self::CycleReservationOverflow
            | Self::QuotaExceeded { .. }
            | Self::CycleReserveRejected { .. } => {
                Some(CostGuardReservePublicKind::ResourceExhausted)
            }
            Self::Store(_) => None,
        }
    }
}

impl From<CostGuardReserveError> for InternalError {
    fn from(err: CostGuardReserveError) -> Self {
        match err {
            CostGuardReserveError::Store(err) => err,
            other => Self::ops(InternalErrorOrigin::Ops, other.to_string()),
        }
    }
}

///
/// CostGuardOps
///
/// Mechanical quota and cycle-reservation facade.
/// Owned by ops and consumed by workflows crossing expensive side effects.
///

pub struct CostGuardOps;

impl CostGuardOps {
    /// reserve
    ///
    /// Reserve quota and cycle budget before a workflow crosses an expensive side-effect boundary.
    pub fn reserve(request: CostGuardRequest) -> Result<CostGuardPermit, CostGuardReserveError> {
        validate_request(&request)?;

        let quota_key = quota_resource_key(&request)?;
        enforce_quota(&quota_key, &request)?;
        let reservation_key = reservation_resource_key(&request)?;
        enforce_cycle_reserve(&reservation_key, &request)?;

        let quota_intent_id = IntentStoreOps::allocate_intent_id()?;
        let quota_record = IntentStoreOps::try_reserve(
            quota_intent_id,
            quota_key.clone(),
            1,
            request.now_secs,
            Some(INTENT_TTL_SECONDS),
            request.now_secs,
        )?;

        let reservation_id = match IntentStoreOps::allocate_intent_id()
            .map_err(CostGuardReserveError::from)
            .and_then(|intent_id| {
                let quantity = u64::try_from(request.cycle_reservation_cycles)
                    .map_err(|_| CostGuardReserveError::CycleReservationOverflow)?;
                IntentStoreOps::try_reserve(
                    intent_id,
                    reservation_key,
                    quantity,
                    request.now_secs,
                    Some(INTENT_TTL_SECONDS),
                    request.now_secs,
                )
                .map(|record| record.id)
                .map_err(CostGuardReserveError::from)
            }) {
            Ok(reservation_id) => reservation_id,
            Err(err) => {
                let _ = IntentStoreOps::abort(quota_record.id);
                return Err(err);
            }
        };

        Ok(CostGuardPermit {
            _cost_class: request.cost_class,
            _quota_key: quota_key,
            reservation_id,
            _payer: request.payer,
            quota_intent_id,
            _private: (),
        })
    }

    /// complete
    ///
    /// Commit both quota and cycle reservation intents after the protected operation succeeds.
    pub fn complete(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        IntentStoreOps::commit_pair_at(permit.quota_intent_id, permit.reservation_id, now_secs)
    }

    /// recover
    ///
    /// Commit quota while releasing cycle reservation after uncertain external effects.
    pub fn recover(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        IntentStoreOps::commit_and_abort_pending_pair_at(
            permit.quota_intent_id,
            permit.reservation_id,
            now_secs,
        )
    }

    /// Recover a failed protected operation and retain any settlement failure
    /// as diagnostic context on the original typed error.
    #[must_use]
    pub fn recover_after_failure(
        permit: &CostGuardPermit,
        now_secs: u64,
        error: InternalError,
    ) -> InternalError {
        match Self::recover(permit, now_secs) {
            Ok(()) => error,
            Err(recovery_error) => error.with_diagnostic_context(format!(
                "cost guard recovery failed for reservation {}: {recovery_error}",
                permit.reservation_id
            )),
        }
    }

    /// abort
    ///
    /// Release all pending cost guard intents for tests that stop before commit/recovery.
    #[cfg(test)]
    pub fn abort(permit: &CostGuardPermit) -> Result<(), InternalError> {
        let _ = IntentStoreOps::abort_intent_if_pending(permit.quota_intent_id)?;
        let _ = IntentStoreOps::abort_intent_if_pending(permit.reservation_id)?;
        Ok(())
    }
}

fn validate_request(request: &CostGuardRequest) -> Result<(), CostGuardReserveError> {
    if request.cost_class == CostClass::None {
        return Err(CostGuardReserveError::UncostedClass);
    }
    if request.quota_window_secs < MIN_QUOTA_WINDOW_SECONDS {
        return Err(CostGuardReserveError::InvalidQuotaWindow);
    }
    if request.max_operations_per_window == 0 {
        return Err(CostGuardReserveError::QuotaRejectsAll);
    }
    if request.cycle_reservation_cycles > u128::from(u64::MAX) {
        return Err(CostGuardReserveError::CycleReservationOverflow);
    }
    Ok(())
}

fn enforce_quota(
    quota_key: &IntentResourceKey,
    request: &CostGuardRequest,
) -> Result<(), CostGuardReserveError> {
    let totals = IntentStoreOps::totals(quota_key);
    let used = totals.committed_qty.saturating_add(totals.reserved_qty);
    if used >= request.max_operations_per_window {
        return Err(CostGuardReserveError::QuotaExceeded {
            command_kind: request.command_kind.as_str().to_string(),
            used,
            max: request.max_operations_per_window,
        });
    }
    Ok(())
}

fn enforce_cycle_reserve(
    reservation_key: &IntentResourceKey,
    request: &CostGuardRequest,
) -> Result<(), CostGuardReserveError> {
    let outstanding = u128::from(IntentStoreOps::totals(reservation_key).reserved_qty);
    let available = request.current_cycle_balance.saturating_sub(outstanding);
    let required = request
        .min_cycles_after_reservation
        .saturating_add(request.cycle_reservation_cycles);

    if available < required {
        return Err(CostGuardReserveError::CycleReserveRejected {
            command_kind: request.command_kind.as_str().to_string(),
            available,
            required,
        });
    }
    Ok(())
}

fn quota_resource_key(
    request: &CostGuardRequest,
) -> Result<IntentResourceKey, CostGuardReserveError> {
    let bucket = request.now_secs / request.quota_window_secs;
    cost_key([
        "cost",
        "quota",
        cost_class_label(request.cost_class),
        &hash_label(request.command_kind.as_str()),
        &hash_principal(request.quota_subject),
        &bucket.to_string(),
    ])
}

fn reservation_resource_key(
    request: &CostGuardRequest,
) -> Result<IntentResourceKey, CostGuardReserveError> {
    cost_key([
        "cost",
        "reserve",
        cost_class_label(request.cost_class),
        &hash_principal(request.payer),
    ])
}

fn cost_key<const N: usize>(
    segments: [&str; N],
) -> Result<IntentResourceKey, CostGuardReserveError> {
    IntentResourceKey::try_new(segments.join(":"))
        .map_err(CostGuardReserveError::ResourceKeyInvalid)
}

const fn cost_class_label(cost_class: CostClass) -> &'static str {
    match cost_class {
        CostClass::None => "none",
        CostClass::RootCanisterSignaturePrepare => "root_canister_signature_prepare",
        CostClass::RootChainKeySigning => "root_chain_key_signing",
        CostClass::IssuerCanisterSignaturePrepare => "issuer_canister_signature_prepare",
        CostClass::ManagementDeployment => "deploy",
        CostClass::ValueTransfer => "transfer",
        CostClass::DurablePublish => "publish",
    }
}

fn hash_label(value: &str) -> String {
    hash_bytes(value.as_bytes())
}

fn hash_principal(value: Principal) -> String {
    hash_bytes(value.as_slice())
}

fn hash_bytes(value: &[u8]) -> String {
    let digest: [u8; 32] = Sha256::digest(value).into();
    digest[..KEY_HASH_BYTES]
        .iter()
        .fold(String::new(), |mut output, byte| {
            let _ = write!(output, "{byte:02x}");
            output
        })
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::stable::intent::IntentStore;

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn reset() {
        IntentStore::reset_for_tests();
    }

    fn request(now_secs: u64) -> CostGuardRequest {
        CostGuardRequest {
            cost_class: CostClass::ManagementDeployment,
            command_kind: CommandKind::new("test.management_deployment.v1").expect("command"),
            quota_subject: p(1),
            payer: p(2),
            now_secs,
            quota_window_secs: 60,
            max_operations_per_window: 1,
            current_cycle_balance: 10,
            cycle_reservation_cycles: 2,
            min_cycles_after_reservation: 5,
        }
    }

    #[test]
    fn reserve_rejects_quota_exhaustion_before_second_per_window_operation() {
        reset();

        let first = CostGuardOps::reserve(request(10)).expect("first reservation");
        CostGuardOps::complete(&first, 10).expect("first completes");

        let err = CostGuardOps::reserve(request(20)).expect_err("same bucket exhausted");
        assert_eq!(
            err.public_kind(),
            Some(CostGuardReservePublicKind::ResourceExhausted)
        );

        CostGuardOps::reserve(request(70)).expect("next bucket allowed");
    }

    #[test]
    fn reserve_rejects_low_cycle_reserve_before_recording_intents() {
        reset();

        let mut low = request(10);
        low.current_cycle_balance = 6;

        let err = CostGuardOps::reserve(low).expect_err("low cycle reserve rejected");
        assert_eq!(
            err.public_kind(),
            Some(CostGuardReservePublicKind::ResourceExhausted)
        );
        assert_eq!(IntentStoreOps::expirable_pending_total().expect("meta"), 0);
    }

    #[test]
    fn abort_releases_pending_quota_and_reservation() {
        reset();

        let permit = CostGuardOps::reserve(request(10)).expect("reservation");
        assert_eq!(
            IntentStoreOps::expirable_pending_total().expect("pending"),
            2
        );

        CostGuardOps::abort(&permit).expect("abort");

        assert_eq!(
            IntentStoreOps::expirable_pending_total().expect("pending"),
            0
        );
    }

    #[test]
    fn complete_rejects_pair_without_partial_quota_commit() {
        reset();

        let permit = CostGuardOps::reserve(request(10)).expect("reservation");
        IntentStoreOps::abort(permit.reservation_id).expect("abort reservation intent");

        CostGuardOps::complete(&permit, 10)
            .expect_err("aborted reservation must reject completion");

        assert_eq!(
            IntentStoreOps::expirable_pending_total().expect("pending"),
            1
        );
        assert!(
            IntentStoreOps::abort_intent_if_pending(permit.quota_intent_id).expect("quota cleanup")
        );
    }

    #[test]
    fn recovery_failure_preserves_original_type_without_partial_settlement() {
        reset();

        let permit = CostGuardOps::reserve(request(10)).expect("reservation");
        IntentStoreOps::abort(permit.quota_intent_id).expect("abort quota intent");
        let original = InternalError::resource_exhausted("protected operation failed");

        let error = CostGuardOps::recover_after_failure(&permit, 10, original);

        assert!(error.is_public_resource_exhausted());
        assert_eq!(
            IntentStoreOps::expirable_pending_total().expect("pending"),
            1
        );
        assert!(
            IntentStoreOps::abort_intent_if_pending(permit.reservation_id)
                .expect("reservation cleanup")
        );
    }

    #[test]
    fn recovery_commits_quota_and_releases_reservation_together() {
        reset();

        let permit = CostGuardOps::reserve(request(10)).expect("reservation");

        CostGuardOps::recover(&permit, 10).expect("recover cost guard");

        assert_eq!(
            IntentStoreOps::expirable_pending_total().expect("pending"),
            0
        );
        IntentStoreOps::commit_at(permit.quota_intent_id, 10).expect("quota intent is committed");
        assert!(
            !IntentStoreOps::abort_intent_if_pending(permit.reservation_id)
                .expect("reservation is terminal")
        );
    }

    #[test]
    fn second_in_flight_reservation_counts_against_cycle_reserve() {
        reset();

        let mut first_req = request(10);
        first_req.max_operations_per_window = 10;
        let _first = CostGuardOps::reserve(first_req).expect("first reservation");

        let mut second_req = request(11);
        second_req.max_operations_per_window = 10;
        second_req.current_cycle_balance = 8;

        let err = CostGuardOps::reserve(second_req).expect_err("outstanding reserve counts");
        assert_eq!(
            err.public_kind(),
            Some(CostGuardReservePublicKind::ResourceExhausted)
        );
    }
}
