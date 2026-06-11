use crate::{
    InternalError,
    cdk::types::Principal,
    dto::error::Error,
    ids::{IntentId, IntentResourceKey},
    ops::{replay::model::CommandKind, storage::intent::IntentStoreOps},
    replay_policy::CostClass,
};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

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
/// CostGuardOps
///
pub struct CostGuardOps;

impl CostGuardOps {
    pub fn reserve(request: CostGuardRequest) -> Result<CostGuardPermit, InternalError> {
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

        let reservation_id = match IntentStoreOps::allocate_intent_id().and_then(|intent_id| {
            let quantity = u64::try_from(request.cycle_reservation_cycles).map_err(|_| {
                InternalError::public(Error::exhausted(
                    "cost guard cycle reservation exceeds supported accounting range",
                ))
            })?;
            IntentStoreOps::try_reserve(
                intent_id,
                reservation_key,
                quantity,
                request.now_secs,
                Some(INTENT_TTL_SECONDS),
                request.now_secs,
            )
            .map(|record| record.id)
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

    pub fn complete(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        IntentStoreOps::commit_at(permit.quota_intent_id, now_secs)?;
        IntentStoreOps::commit_at(permit.reservation_id, now_secs)?;
        Ok(())
    }

    pub fn recover(permit: &CostGuardPermit, now_secs: u64) -> Result<(), InternalError> {
        IntentStoreOps::commit_at(permit.quota_intent_id, now_secs)?;
        let _ = IntentStoreOps::abort_intent_if_pending(permit.reservation_id)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn abort(permit: &CostGuardPermit) -> Result<(), InternalError> {
        let _ = IntentStoreOps::abort_intent_if_pending(permit.quota_intent_id)?;
        let _ = IntentStoreOps::abort_intent_if_pending(permit.reservation_id)?;
        Ok(())
    }
}

fn validate_request(request: &CostGuardRequest) -> Result<(), InternalError> {
    if request.cost_class == CostClass::None {
        return Err(InternalError::public(Error::invalid(
            "cost guard requires a costed operation class",
        )));
    }
    if request.quota_window_secs < MIN_QUOTA_WINDOW_SECONDS {
        return Err(InternalError::public(Error::invalid(
            "cost guard quota window must be greater than zero",
        )));
    }
    if request.max_operations_per_window == 0 {
        return Err(InternalError::public(Error::exhausted(
            "cost guard quota rejects all new operations",
        )));
    }
    if request.cycle_reservation_cycles > u128::from(u64::MAX) {
        return Err(InternalError::public(Error::exhausted(
            "cost guard cycle reservation exceeds supported accounting range",
        )));
    }
    Ok(())
}

fn enforce_quota(
    quota_key: &IntentResourceKey,
    request: &CostGuardRequest,
) -> Result<(), InternalError> {
    let totals = IntentStoreOps::totals_at(quota_key, request.now_secs);
    let used = totals.committed_qty.saturating_add(totals.reserved_qty);
    if used >= request.max_operations_per_window {
        return Err(InternalError::public(Error::exhausted(format!(
            "cost guard quota exceeded for {}: used={} max={}",
            request.command_kind.as_str(),
            used,
            request.max_operations_per_window
        ))));
    }
    Ok(())
}

fn enforce_cycle_reserve(
    reservation_key: &IntentResourceKey,
    request: &CostGuardRequest,
) -> Result<(), InternalError> {
    let outstanding =
        u128::from(IntentStoreOps::totals_at(reservation_key, request.now_secs).reserved_qty);
    let available = request.current_cycle_balance.saturating_sub(outstanding);
    let required = request
        .min_cycles_after_reservation
        .saturating_add(request.cycle_reservation_cycles);

    if available < required {
        return Err(InternalError::public(Error::exhausted(format!(
            "cost guard cycle reserve rejected {}: available={} required={}",
            request.command_kind.as_str(),
            available,
            required
        ))));
    }
    Ok(())
}

fn quota_resource_key(request: &CostGuardRequest) -> Result<IntentResourceKey, InternalError> {
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
) -> Result<IntentResourceKey, InternalError> {
    cost_key([
        "cost",
        "reserve",
        cost_class_label(request.cost_class),
        &hash_principal(request.payer),
    ])
}

fn cost_key<const N: usize>(segments: [&str; N]) -> Result<IntentResourceKey, InternalError> {
    IntentResourceKey::try_new(segments.join(":")).map_err(|err| {
        InternalError::public(Error::invalid(format!(
            "cost guard resource key is invalid: {err}"
        )))
    })
}

const fn cost_class_label(cost_class: CostClass) -> &'static str {
    match cost_class {
        CostClass::None => "none",
        CostClass::ThresholdEcdsaSign => "ecdsa",
        CostClass::RootCanisterSignaturePrepare => "root_canister_signature_prepare",
        CostClass::IssuerCanisterSignaturePrepare => "issuer_canister_signature_prepare",
        CostClass::ShardTokenSign => "shard_token_sign",
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dto::error::ErrorCode, storage::stable::intent::IntentStore};

    fn p(id: u8) -> Principal {
        Principal::from_slice(&[id; 29])
    }

    fn reset() {
        IntentStore::reset_for_tests();
    }

    fn request(now_secs: u64) -> CostGuardRequest {
        CostGuardRequest {
            cost_class: CostClass::ThresholdEcdsaSign,
            command_kind: CommandKind::new("auth.issue_delegation_proof.v1").expect("command"),
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
            err.public_error().expect("quota rejection is public").code,
            ErrorCode::ResourceExhausted
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
            err.public_error()
                .expect("cycle reserve rejection is public")
                .code,
            ErrorCode::ResourceExhausted
        );
        assert_eq!(IntentStoreOps::pending_total().expect("meta"), 0);
    }

    #[test]
    fn abort_releases_pending_quota_and_reservation() {
        reset();

        let permit = CostGuardOps::reserve(request(10)).expect("reservation");
        assert_eq!(IntentStoreOps::pending_total().expect("pending"), 2);

        CostGuardOps::abort(&permit).expect("abort");

        assert_eq!(IntentStoreOps::pending_total().expect("pending"), 0);
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
            err.public_error()
                .expect("cycle reserve rejection is public")
                .code,
            ErrorCode::ResourceExhausted
        );
    }
}
