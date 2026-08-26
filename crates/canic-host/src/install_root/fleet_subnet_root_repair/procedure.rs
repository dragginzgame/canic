//! Module: install_root::fleet_subnet_root_repair::procedure
//!
//! Responsibility: durably execute and reconcile the one provisionally authorized retained Root
//! repair.
//! Does not own: general upgrades, fresh installation, pool allocation, or ICP conversion.
//! Boundary: one immutable repair operation may upgrade its exact Root, fund and re-inspect its
//! exact retained imported pool Canister, then become eligible for terminal receipt publication.

use super::{
    ResolvedRetainedRootRepair, RetainedRootRepairAuthorityV1, require_durable_terminal_receipt,
};
use crate::{
    durable_io::{
        BoundedRegularFileReadError, CanonicalJsonEncodeError, CanonicalJsonStyle,
        ExactReplaceError, RegularFileLockError, RegularFileReadError, create_new_bytes,
        encode_canonical_json, lock_regular_file_with_parents, read_optional_bounded_regular_bytes,
        replace_bytes_exact, write_bytes,
    },
    icp,
    install_root::{
        commands::icp_canister_upgrade_binary_args_command,
        fleet_install_recovery_bundle::FleetInstallRecoveryBundleCheckpoint,
        icp_context::InstallIcpContext,
        operations::{call_with_arg, observe_module_hash, query_with_arg},
    },
    protocol_binding::ResolvedProtocolBinding,
};
use candid::{CandidType, Principal};
use canic_core::{
    dto::pool::{
        CanisterPoolAsset, CanisterPoolAssetOrigin, CanisterPoolAssetStatus, CanisterPoolResponse,
        CanisterPoolStatusRequest, PoolCanisterRequest, PoolImportResponse,
    },
    protocol,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt::Write as _, io, path::PathBuf};
use thiserror::Error as ThisError;

const REPAIR_OPERATION_FILE: &str = "root-repair-operation.json";
const REPAIR_OPERATION_LOCK_FILE: &str = "root-repair-operation.lock";
const REPAIR_UPGRADE_ARGS_FILE: &str = "root-repair-upgrade-args.bin";
const REPAIR_OPERATION_SCHEMA_VERSION: u32 = 1;
const MAX_REPAIR_OPERATION_BYTES: usize = 32 * 1024;
const MAX_COMMAND_RECEIPT_BYTES: usize = 4 * 1024;
const MAX_FUNDING_ATTEMPTS: usize = 4;
const POOL_PAGE_LIMIT: u16 = 256;
const MAX_POOL_PAGES: usize = 17;

#[derive(CandidType)]
enum RootCommandFragment {
    ImportPoolCanister(PoolCanisterRequest),
}

#[derive(CandidType, Deserialize)]
enum RootCommandResponseFragment {
    ImportPoolCanister(PoolImportResponse),
}

#[derive(CandidType)]
enum RootStatusRequestFragment {
    Pool(CanisterPoolStatusRequest),
}

#[derive(CandidType, Deserialize)]
enum RootStatusResponseFragment {
    Pool(CanisterPoolResponse),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RetainedRootRepairOperationPhaseV1 {
    Planned,
    UpgradeInFlight,
    UpgradeVerified,
    TopUpInFlight,
    ReinspectionInFlight,
    FundingRequired,
    AssetReady,
    Adopted,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RetainedRootRepairFundingAttemptV1 {
    sequence: u8,
    actual_cycles_before: u128,
    required_cycles: u128,
    deficit_cycles: u128,
    fee_cycles: u128,
    margin_cycles: u128,
    requested_cycles: u128,
    maximum_operator_debit_cycles: u128,
    operator_cycles_before: u128,
    command_receipt: Option<String>,
    actual_cycles_after: Option<u128>,
    operator_cycles_after: Option<u128>,
    operator_debit_cycles: Option<u128>,
    asset_credit_cycles: Option<u128>,
    observed_fee_and_burn_cycles: Option<u128>,
    retained_margin_cycles: Option<u128>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::install_root) struct RetainedRootRepairOperationV1 {
    schema_version: u32,
    repair_operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    pool_canister: Principal,
    upgrade_predecessor_module_sha256: [u8; 32],
    successor_module_sha256: [u8; 32],
    required_pool_cycles: u128,
    top_up_fee_cycles: u128,
    top_up_margin_cycles: u128,
    phase: RetainedRootRepairOperationPhaseV1,
    upgrade_command_receipt: Option<String>,
    funding_attempts: Vec<RetainedRootRepairFundingAttemptV1>,
    final_actual_cycles: Option<u128>,
}

#[cfg(test)]
impl RetainedRootRepairOperationV1 {
    pub(super) fn test_funding_reconciliation(
        &self,
    ) -> Result<(usize, u128, u128, u128), RetainedRootRepairProcedureError> {
        Ok((
            self.funding_attempts.len(),
            sum_observed(&self.funding_attempts, |attempt| {
                attempt.operator_debit_cycles
            })?,
            sum_observed(&self.funding_attempts, |attempt| {
                attempt.asset_credit_cycles
            })?,
            sum_observed(&self.funding_attempts, |attempt| {
                attempt.observed_fee_and_burn_cycles
            })?,
        ))
    }
}

#[derive(Debug, ThisError)]
pub(in crate::install_root) enum RetainedRootRepairProcedureError {
    #[error("retained Root repair operation already has different immutable authority: {path}")]
    ConflictingAuthority { path: PathBuf },

    #[error("invalid retained Root repair operation {path}: {reason}")]
    InvalidDocument { path: PathBuf, reason: String },

    #[error("retained Root repair operation is not a regular no-follow file: {path}")]
    UnsafeFile { path: PathBuf },

    #[error("retained Root repair operation lock is not a regular no-follow file: {path}")]
    UnsafeLock { path: PathBuf },

    #[error("failed to access retained Root repair operation {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("retained Root repair arithmetic overflowed")]
    ArithmeticOverflow,

    #[error(
        "retained Root repair live module is neither its exact predecessor nor authorized successor"
    )]
    ModuleDrift,

    #[error("retained Root repair upgrade outcome is uncertain; no second upgrade was attempted")]
    UpgradeOutcomeUnknown,

    #[error("retained Root repair upgrade command receipt exceeds 4 KiB")]
    UpgradeReceiptTooLarge,

    #[error("retained Root repair top-up command receipt exceeds 4 KiB")]
    TopUpReceiptTooLarge,

    #[error("retained Root repair pool status omitted the exact retained asset")]
    PoolAssetMissing,

    #[error("retained Root repair asset is not the exact empty imported Ready/reset asset")]
    PoolAssetIneligible,

    #[error("retained Root repair pool status pagination exceeded its bound")]
    PoolPaginationExceeded,

    #[error("retained Root repair top-up outcome is uncertain; no second payment was attempted")]
    TopUpOutcomeUnknown,

    #[error("retained Root repair exceeded four exact funding reconciliation attempts")]
    FundingAttemptBoundExceeded,

    #[error(transparent)]
    FundingReviewRequired(Box<RetainedRootRepairFundingReviewRequired>),

    #[error("retained Root repair funding authorization digest is missing or stale")]
    FundingAuthorizationMismatch,

    #[error(
        "retained Root repair requires {required_cycles} operator cycles for the next exact top-up but only {available_cycles} are available"
    )]
    InsufficientOperatorCycles {
        required_cycles: u128,
        available_cycles: u128,
    },

    #[error("retained Root repair reinspection returned an unexpected response")]
    ReinspectionResponseMismatch,
}

#[derive(Debug, ThisError)]
#[error(
    "retained Root repair funding requires operator review: actual={actual_cycles} required={required_cycles} deficit={deficit_cycles} requested={requested_cycles} fee={fee_cycles} margin={margin_cycles} next_maximum_operator_debit={next_maximum_operator_debit_cycles} cumulative_remaining_authority={cumulative_remaining_authority_cycles} remaining_attempts={remaining_attempts} authorization_digest={authorization_digest}; Root retained {failure_reason}"
)]
pub(in crate::install_root) struct RetainedRootRepairFundingReviewRequired {
    actual_cycles: u128,
    required_cycles: u128,
    deficit_cycles: u128,
    requested_cycles: u128,
    fee_cycles: u128,
    margin_cycles: u128,
    next_maximum_operator_debit_cycles: u128,
    cumulative_remaining_authority_cycles: u128,
    remaining_attempts: usize,
    authorization_digest: String,
    failure_reason: String,
}

/// Execute or reconcile every effect admitted by one exact retained Root repair receipt.
pub(in crate::install_root) fn execute_retained_root_repair(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    resolved: &ResolvedRetainedRootRepair,
    successor_wasm: &std::path::Path,
    funding_authorization_digest: Option<&str>,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let lock_path = resolved.path.with_file_name(REPAIR_OPERATION_LOCK_FILE);
    let _lock = lock_operation(&lock_path)?;
    let mut current = create_or_load_operation(&operation_path, &resolved.authority)?;
    recovery_bundle.checkpoint()?;

    current = reconcile_upgrade(
        icp_context,
        &operation_path,
        current,
        &resolved.authority,
        successor_wasm,
        recovery_bundle,
    )?;
    current = reconcile_pool_asset(
        icp_context,
        root_binding,
        &operation_path,
        current,
        &resolved.authority,
        funding_authorization_digest,
        recovery_bundle,
    )?;
    print_reconciliation(&current)?;
    Ok(current)
}

/// Converge the operation after an exact immutable receipt won its create-new race.
///
/// This boundary performs no canister, Ledger, pool or receipt effect. It exists only for the
/// interruption between receipt publication and the final local operation replacement.
pub(in crate::install_root) fn reconcile_published_retained_root_repair(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<(), RetainedRootRepairProcedureError> {
    require_durable_terminal_receipt(resolved)
        .map_err(|error| invalid(&resolved.path, error.to_string()))?;
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let lock_path = resolved.path.with_file_name(REPAIR_OPERATION_LOCK_FILE);
    let _lock = lock_operation(&lock_path)?;
    let current = load_optional_operation(&operation_path)?.ok_or_else(|| {
        invalid(
            &operation_path,
            "published receipt omitted its repair operation",
        )
    })?;
    validate_operation(&operation_path, &current, &resolved.authority)?;
    match current.phase {
        RetainedRootRepairOperationPhaseV1::AssetReady => {
            let mut adopted = current;
            adopted.phase = RetainedRootRepairOperationPhaseV1::Adopted;
            replace_operation(&operation_path, &adopted)
        }
        RetainedRootRepairOperationPhaseV1::Adopted => Ok(()),
        _ => Err(invalid(
            &operation_path,
            "published receipt precedes exact terminal repair evidence",
        )),
    }
}

pub(in crate::install_root) fn retained_root_repair_operation_path(
    journal_path: &std::path::Path,
) -> PathBuf {
    journal_path.with_file_name(REPAIR_OPERATION_FILE)
}

pub(in crate::install_root) fn validate_recovery_bundle_repair_operation_bytes(
    path: &std::path::Path,
    bytes: &[u8],
    authority: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairProcedureError> {
    let operation = serde_json::from_slice::<RetainedRootRepairOperationV1>(bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    validate_operation(path, &operation, authority)?;
    if encode_operation(path, &operation)? != bytes {
        return Err(invalid(path, "repair operation bytes are not canonical"));
    }
    Ok(())
}

/// Read-only funding exposure retained by one exact repair operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(in crate::install_root) struct RetainedRootRepairFundingExposureV1 {
    pub repair_operation_id: String,
    pub fleet_subnet_root: String,
    pub pool_canister: String,
    pub operator: String,
    pub phase: String,
    pub actual_cycles: Option<u128>,
    pub required_cycles: u128,
    pub deficit_cycles: Option<u128>,
    pub next_requested_cycles: Option<u128>,
    pub next_maximum_operator_debit_cycles: Option<u128>,
    pub cumulative_remaining_authority_cycles: u128,
    pub completed_funding_attempts: usize,
    pub remaining_funding_attempts: usize,
    pub authorization_digest: Option<String>,
    pub operator_available_cycles: Option<u128>,
    pub operator_balance_sufficient: Option<bool>,
}

/// Inspect an existing operation without creating or replacing any retained evidence.
pub(in crate::install_root) fn inspect_retained_root_repair_funding(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<Option<RetainedRootRepairFundingExposureV1>, RetainedRootRepairProcedureError> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let operation = load_optional_operation(&operation_path)?
        .unwrap_or_else(|| planned_operation(&resolved.authority));
    validate_operation(&operation_path, &operation, &resolved.authority)?;
    funding_exposure(&operation, &resolved.authority).map(Some)
}

/// Observe an existing repair's exact pool balance without issuing a repair or payment effect.
///
/// A retained `reinspection_in_flight` checkpoint may be behind a live failed pool row when the
/// protected reinspection crossed the funding threshold before the host recorded its result. This
/// preflight boundary may retain that exact query observation as `funding_required`; it never
/// creates a repair operation, invokes the protected reinspection command, upgrades a Root, or
/// transfers cycles.
pub(in crate::install_root) fn preflight_retained_root_repair_funding(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    resolved: &ResolvedRetainedRootRepair,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairFundingExposureV1, Box<dyn std::error::Error>> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let lock_path = resolved.path.with_file_name(REPAIR_OPERATION_LOCK_FILE);
    let _lock = lock_operation(&lock_path)?;
    let Some(mut current) = load_optional_operation(&operation_path)? else {
        return funding_exposure(&planned_operation(&resolved.authority), &resolved.authority)
            .map_err(Into::into);
    };
    validate_operation(&operation_path, &current, &resolved.authority)?;

    let may_observe_exact_balance = matches!(
        current.phase,
        RetainedRootRepairOperationPhaseV1::ReinspectionInFlight
            | RetainedRootRepairOperationPhaseV1::FundingRequired
    ) && !has_incomplete_funding_attempt(&current);
    if may_observe_exact_balance {
        let asset = query_pool_asset(icp_context, root_binding, &resolved.authority)?;
        require_repairable_asset(&asset)?;
        let actual_cycles = asset.cycles.to_u128();
        if actual_cycles < resolved.authority.required_pool_cycles {
            if current.phase != RetainedRootRepairOperationPhaseV1::FundingRequired
                || current.final_actual_cycles != Some(actual_cycles)
            {
                current.phase = RetainedRootRepairOperationPhaseV1::FundingRequired;
                current.final_actual_cycles = Some(actual_cycles);
                replace_operation(&operation_path, &current)?;
                recovery_bundle.checkpoint()?;
            }
        } else if current.phase == RetainedRootRepairOperationPhaseV1::FundingRequired {
            // An external top-up invalidates the retained debit digest. The protected Root
            // reinspection still owns the transition to Ready, so preflight records only that the
            // prior exact funding request is no longer current.
            current.phase = RetainedRootRepairOperationPhaseV1::ReinspectionInFlight;
            current.final_actual_cycles = None;
            replace_operation(&operation_path, &current)?;
            recovery_bundle.checkpoint()?;
        }
    }

    let mut exposure = funding_exposure(&current, &resolved.authority)?;
    if let Some(required_cycles) = exposure.next_maximum_operator_debit_cycles {
        let available_cycles = icp_context.cli().identity_cycles_balance()?;
        exposure.operator_available_cycles = Some(available_cycles);
        exposure.operator_balance_sufficient = Some(available_cycles >= required_cycles);
    }
    Ok(exposure)
}

#[cfg(test)]
pub(super) fn write_asset_ready_test_operation(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<(), RetainedRootRepairProcedureError> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let mut operation = create_or_load_operation(&operation_path, &resolved.authority)?;
    operation.phase = RetainedRootRepairOperationPhaseV1::AssetReady;
    operation.final_actual_cycles = Some(resolved.authority.required_pool_cycles);
    replace_operation(&operation_path, &operation)
}

#[cfg(test)]
pub(super) fn write_reinspection_in_flight_test_operation(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<(), RetainedRootRepairProcedureError> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    let mut operation = create_or_load_operation(&operation_path, &resolved.authority)?;
    operation.phase = RetainedRootRepairOperationPhaseV1::ReinspectionInFlight;
    operation.final_actual_cycles = None;
    replace_operation(&operation_path, &operation)
}

#[cfg(test)]
pub(super) fn test_operation_is_adopted(
    resolved: &ResolvedRetainedRootRepair,
) -> Result<bool, RetainedRootRepairProcedureError> {
    let operation_path = resolved.path.with_file_name(REPAIR_OPERATION_FILE);
    Ok(load_optional_operation(&operation_path)?
        .is_some_and(|operation| operation.phase == RetainedRootRepairOperationPhaseV1::Adopted))
}

fn reconcile_upgrade(
    icp_context: &InstallIcpContext,
    path: &std::path::Path,
    mut current: RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
    successor_wasm: &std::path::Path,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    let observed = observe_module_hash(icp_context.cli(), receipt.fleet_subnet_root)?;
    if observed == Some(receipt.successor_module_sha256) {
        if matches!(
            current.phase,
            RetainedRootRepairOperationPhaseV1::Planned
                | RetainedRootRepairOperationPhaseV1::UpgradeInFlight
        ) {
            current.phase = RetainedRootRepairOperationPhaseV1::UpgradeVerified;
            replace_operation(path, &current)?;
            recovery_bundle.checkpoint()?;
        }
        return Ok(current);
    }
    if observed != Some(receipt.upgrade_predecessor_module_sha256) {
        return Err(RetainedRootRepairProcedureError::ModuleDrift.into());
    }
    if current.phase == RetainedRootRepairOperationPhaseV1::UpgradeInFlight {
        return Err(RetainedRootRepairProcedureError::UpgradeOutcomeUnknown.into());
    }
    if current.phase != RetainedRootRepairOperationPhaseV1::Planned {
        return Err(RetainedRootRepairProcedureError::ModuleDrift.into());
    }

    current.phase = RetainedRootRepairOperationPhaseV1::UpgradeInFlight;
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    let args_path = resolved_sibling(path, REPAIR_UPGRADE_ARGS_FILE);
    write_bytes(&args_path, &candid::encode_one(())?)?;
    let mut command = icp_canister_upgrade_binary_args_command(
        icp_context,
        receipt.fleet_subnet_root,
        successor_wasm,
        &args_path,
    );
    let command_result = icp::run_output_with_stderr(&mut command);
    let observed = observe_module_hash(icp_context.cli(), receipt.fleet_subnet_root)?;
    if observed != Some(receipt.successor_module_sha256) {
        return Err(match command_result {
            Ok(_) | Err(_) => RetainedRootRepairProcedureError::UpgradeOutcomeUnknown.into(),
        });
    }
    let command_receipt = bounded_command_receipt(
        command_result.unwrap_or_else(|error| error.to_string()),
        RetainedRootRepairProcedureError::UpgradeReceiptTooLarge,
    )?;
    current.upgrade_command_receipt = Some(command_receipt);
    current.phase = RetainedRootRepairOperationPhaseV1::UpgradeVerified;
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    Ok(current)
}

fn reconcile_pool_asset(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    path: &std::path::Path,
    mut current: RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
    funding_authorization_digest: Option<&str>,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    if matches!(
        current.phase,
        RetainedRootRepairOperationPhaseV1::AssetReady
            | RetainedRootRepairOperationPhaseV1::Adopted
    ) {
        require_ready_asset(
            &query_pool_asset(icp_context, root_binding, receipt)?,
            receipt.required_pool_cycles,
        )?;
        return Ok(current);
    }

    let before = query_pool_asset(icp_context, root_binding, receipt)?;
    require_repairable_asset(&before)?;
    if has_incomplete_funding_attempt(&current) {
        return reconcile_uncertain_top_up(
            icp_context,
            root_binding,
            path,
            current,
            receipt,
            recovery_bundle,
        );
    }
    if before.cycles.to_u128() >= receipt.required_pool_cycles {
        current.phase = RetainedRootRepairOperationPhaseV1::ReinspectionInFlight;
        replace_operation(path, &current)?;
        recovery_bundle.checkpoint()?;
        reinspect_pool_asset(icp_context, root_binding, receipt)?;
        let ready = query_pool_asset(icp_context, root_binding, receipt)?;
        if ready.cycles.to_u128() < receipt.required_pool_cycles {
            return retain_funding_required(path, current, &ready, receipt, recovery_bundle);
        }
        require_ready_asset(&ready, receipt.required_pool_cycles)?;
        current.final_actual_cycles = Some(ready.cycles.to_u128());
        current.phase = RetainedRootRepairOperationPhaseV1::AssetReady;
        replace_operation(path, &current)?;
        recovery_bundle.checkpoint()?;
        return Ok(current);
    }

    if current.funding_attempts.len() >= MAX_FUNDING_ATTEMPTS {
        return Err(RetainedRootRepairProcedureError::FundingAttemptBoundExceeded.into());
    }

    if current.phase != RetainedRootRepairOperationPhaseV1::FundingRequired
        || current.final_actual_cycles != Some(before.cycles.to_u128())
    {
        return retain_funding_required(path, current, &before, receipt, recovery_bundle);
    }

    let exposure = funding_exposure(&current, receipt)?;
    if funding_authorization_digest != exposure.authorization_digest.as_deref() {
        return Err(RetainedRootRepairProcedureError::FundingAuthorizationMismatch.into());
    }

    let plan = begin_funding_attempt(
        icp_context,
        path,
        &mut current,
        before.cycles.to_u128(),
        receipt,
        recovery_bundle,
    )?;

    let command_result = icp_context
        .cli()
        .canister_top_up_output(&receipt.pool_canister.to_text(), plan.requested_cycles);
    if let Ok(output) = &command_result {
        let bounded = bounded_command_receipt(
            output.clone(),
            RetainedRootRepairProcedureError::TopUpReceiptTooLarge,
        )?;
        current
            .funding_attempts
            .last_mut()
            .expect("funding intent was retained")
            .command_receipt = Some(bounded);
    }
    current.phase = RetainedRootRepairOperationPhaseV1::ReinspectionInFlight;
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    reinspect_pool_asset(icp_context, root_binding, receipt)?;
    current = reconcile_top_up_observation(
        icp_context,
        root_binding,
        path,
        current,
        receipt,
        command_result.is_ok(),
        recovery_bundle,
    )?;
    if current.phase == RetainedRootRepairOperationPhaseV1::AssetReady {
        return Ok(current);
    }
    let after = query_pool_asset(icp_context, root_binding, receipt)?;
    retain_funding_required(path, current, &after, receipt, recovery_bundle)
}

fn retain_funding_required(
    path: &std::path::Path,
    mut current: RetainedRootRepairOperationV1,
    asset: &CanisterPoolAsset,
    receipt: &RetainedRootRepairAuthorityV1,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    require_repairable_asset(asset)?;
    if asset.cycles.to_u128() >= receipt.required_pool_cycles {
        return Err(RetainedRootRepairProcedureError::PoolAssetIneligible.into());
    }
    current.final_actual_cycles = Some(asset.cycles.to_u128());
    current.phase = RetainedRootRepairOperationPhaseV1::FundingRequired;
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    let exposure = funding_exposure(&current, receipt)?;
    let plan = funding_plan(
        asset.cycles.to_u128(),
        receipt.required_pool_cycles,
        receipt.top_up_fee_cycles,
        receipt.top_up_margin_cycles,
    )?;
    Err(
        RetainedRootRepairProcedureError::FundingReviewRequired(Box::new(
            RetainedRootRepairFundingReviewRequired {
                actual_cycles: plan.actual_cycles,
                required_cycles: plan.required_cycles,
                deficit_cycles: plan.deficit_cycles,
                requested_cycles: plan.requested_cycles,
                fee_cycles: plan.fee_cycles,
                margin_cycles: plan.margin_cycles,
                next_maximum_operator_debit_cycles: plan.maximum_operator_debit_cycles,
                cumulative_remaining_authority_cycles: exposure
                    .cumulative_remaining_authority_cycles,
                remaining_attempts: exposure.remaining_funding_attempts,
                authorization_digest: exposure
                    .authorization_digest
                    .expect("funding-required exposure has an authorization digest"),
                failure_reason: repairable_failure_reason(asset),
            },
        ))
        .into(),
    )
}

fn has_incomplete_funding_attempt(operation: &RetainedRootRepairOperationV1) -> bool {
    operation
        .funding_attempts
        .last()
        .is_some_and(|attempt| attempt.actual_cycles_after.is_none())
}

fn begin_funding_attempt(
    icp_context: &InstallIcpContext,
    path: &std::path::Path,
    current: &mut RetainedRootRepairOperationV1,
    actual_cycles: u128,
    receipt: &RetainedRootRepairAuthorityV1,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairFundingPlan, Box<dyn std::error::Error>> {
    let operator_cycles_before = icp_context.cli().identity_cycles_balance()?;
    let plan = funding_plan(
        actual_cycles,
        receipt.required_pool_cycles,
        receipt.top_up_fee_cycles,
        receipt.top_up_margin_cycles,
    )?;
    if operator_cycles_before < plan.maximum_operator_debit_cycles {
        return Err(
            RetainedRootRepairProcedureError::InsufficientOperatorCycles {
                required_cycles: plan.maximum_operator_debit_cycles,
                available_cycles: operator_cycles_before,
            }
            .into(),
        );
    }
    let sequence = u8::try_from(current.funding_attempts.len() + 1)
        .map_err(|_| RetainedRootRepairProcedureError::FundingAttemptBoundExceeded)?;
    current
        .funding_attempts
        .push(RetainedRootRepairFundingAttemptV1 {
            sequence,
            actual_cycles_before: plan.actual_cycles,
            required_cycles: plan.required_cycles,
            deficit_cycles: plan.deficit_cycles,
            fee_cycles: plan.fee_cycles,
            margin_cycles: plan.margin_cycles,
            requested_cycles: plan.requested_cycles,
            maximum_operator_debit_cycles: plan.maximum_operator_debit_cycles,
            operator_cycles_before,
            command_receipt: None,
            actual_cycles_after: None,
            operator_cycles_after: None,
            operator_debit_cycles: None,
            asset_credit_cycles: None,
            observed_fee_and_burn_cycles: None,
            retained_margin_cycles: None,
        });
    current.phase = RetainedRootRepairOperationPhaseV1::TopUpInFlight;
    replace_operation(path, current)?;
    recovery_bundle.checkpoint()?;
    Ok(plan)
}

fn reconcile_uncertain_top_up(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    path: &std::path::Path,
    mut current: RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    current.phase = RetainedRootRepairOperationPhaseV1::ReinspectionInFlight;
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    reinspect_pool_asset(icp_context, root_binding, receipt)?;
    reconcile_top_up_observation(
        icp_context,
        root_binding,
        path,
        current,
        receipt,
        false,
        recovery_bundle,
    )
}

fn reconcile_top_up_observation(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    path: &std::path::Path,
    mut current: RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
    command_succeeded: bool,
    recovery_bundle: &FleetInstallRecoveryBundleCheckpoint<'_>,
) -> Result<RetainedRootRepairOperationV1, Box<dyn std::error::Error>> {
    let after = query_pool_asset(icp_context, root_binding, receipt)?;
    let operator_cycles_after = icp_context.cli().identity_cycles_balance()?;
    let attempt = current
        .funding_attempts
        .last_mut()
        .ok_or_else(|| invalid(path, "top-up phase omitted its exact funding intent"))?;
    let observation = funding_observation(
        attempt,
        after.cycles.to_u128(),
        operator_cycles_after,
        command_succeeded,
    )?;
    attempt.actual_cycles_after = Some(after.cycles.to_u128());
    attempt.operator_cycles_after = Some(operator_cycles_after);
    attempt.operator_debit_cycles = Some(observation.operator_debit_cycles);
    attempt.asset_credit_cycles = Some(observation.asset_credit_cycles);
    attempt.observed_fee_and_burn_cycles = Some(observation.observed_fee_and_burn_cycles);
    attempt.retained_margin_cycles = Some(
        after
            .cycles
            .to_u128()
            .saturating_sub(receipt.required_pool_cycles),
    );
    if after.cycles.to_u128() >= receipt.required_pool_cycles
        && after.status == CanisterPoolAssetStatus::Ready
    {
        current.final_actual_cycles = Some(after.cycles.to_u128());
        current.phase = RetainedRootRepairOperationPhaseV1::AssetReady;
    } else {
        require_repairable_asset(&after)?;
        current.final_actual_cycles = Some(after.cycles.to_u128());
        current.phase = RetainedRootRepairOperationPhaseV1::FundingRequired;
    }
    replace_operation(path, &current)?;
    recovery_bundle.checkpoint()?;
    Ok(current)
}

fn reinspect_pool_asset(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<(), Box<dyn std::error::Error>> {
    let response: RootCommandResponseFragment = call_with_arg(
        icp_context.cli(),
        root_binding,
        receipt.fleet_subnet_root,
        protocol::CANIC_COMMAND,
        &RootCommandFragment::ImportPoolCanister(PoolCanisterRequest {
            canister_id: receipt.pool_canister,
        }),
    )?;
    let RootCommandResponseFragment::ImportPoolCanister(response) = response;
    match response {
        PoolImportResponse::Imported { canister_id }
        | PoolImportResponse::ResetFailed { canister_id, .. }
            if canister_id == receipt.pool_canister =>
        {
            Ok(())
        }
        _ => Err(RetainedRootRepairProcedureError::ReinspectionResponseMismatch.into()),
    }
}

fn query_pool_asset(
    icp_context: &InstallIcpContext,
    root_binding: &ResolvedProtocolBinding,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<CanisterPoolAsset, Box<dyn std::error::Error>> {
    let mut start_after = None;
    for _ in 0..MAX_POOL_PAGES {
        let response: RootStatusResponseFragment = query_with_arg(
            icp_context.cli(),
            root_binding,
            receipt.fleet_subnet_root,
            protocol::CANIC_STATUS,
            &RootStatusRequestFragment::Pool(CanisterPoolStatusRequest {
                start_after,
                limit: POOL_PAGE_LIMIT,
            }),
        )?;
        let RootStatusResponseFragment::Pool(page) = response;
        if let Some(asset) = page
            .entries
            .into_iter()
            .find(|asset| asset.canister_id == receipt.pool_canister)
        {
            return Ok(asset);
        }
        let Some(next) = page.next_start_after else {
            return Err(RetainedRootRepairProcedureError::PoolAssetMissing.into());
        };
        if start_after.is_some_and(|previous| next <= previous) {
            return Err(RetainedRootRepairProcedureError::PoolPaginationExceeded.into());
        }
        start_after = Some(next);
    }
    Err(RetainedRootRepairProcedureError::PoolPaginationExceeded.into())
}

fn require_repairable_asset(
    asset: &CanisterPoolAsset,
) -> Result<(), RetainedRootRepairProcedureError> {
    let repairable_state = matches!(
        asset.status,
        CanisterPoolAssetStatus::Ready
            | CanisterPoolAssetStatus::PendingReset
            | CanisterPoolAssetStatus::Failed { .. }
    );
    if asset.origin == CanisterPoolAssetOrigin::Imported && repairable_state {
        Ok(())
    } else {
        Err(RetainedRootRepairProcedureError::PoolAssetIneligible)
    }
}

fn require_ready_asset(
    asset: &CanisterPoolAsset,
    required_cycles: u128,
) -> Result<(), RetainedRootRepairProcedureError> {
    if asset.origin == CanisterPoolAssetOrigin::Imported
        && asset.status == CanisterPoolAssetStatus::Ready
        && asset.cycles.to_u128() >= required_cycles
    {
        Ok(())
    } else {
        Err(RetainedRootRepairProcedureError::PoolAssetIneligible)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetainedRootRepairFundingPlan {
    actual_cycles: u128,
    required_cycles: u128,
    deficit_cycles: u128,
    fee_cycles: u128,
    margin_cycles: u128,
    requested_cycles: u128,
    maximum_operator_debit_cycles: u128,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RetainedRootRepairFundingObservation {
    operator_debit_cycles: u128,
    asset_credit_cycles: u128,
    observed_fee_and_burn_cycles: u128,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct RetainedRootRepairFundingAuthorizationV1 {
    schema_version: u16,
    repair_operation_id: [u8; 32],
    fleet_subnet_root: Principal,
    pool_canister: Principal,
    operator: Principal,
    actual_cycles: u128,
    required_cycles: u128,
    deficit_cycles: u128,
    requested_cycles: u128,
    maximum_operator_debit_cycles: u128,
    cumulative_remaining_authority_cycles: u128,
    completed_funding_attempts: usize,
    remaining_funding_attempts: usize,
}

fn funding_exposure(
    operation: &RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<RetainedRootRepairFundingExposureV1, RetainedRootRepairProcedureError> {
    let terminal = matches!(
        operation.phase,
        RetainedRootRepairOperationPhaseV1::AssetReady
            | RetainedRootRepairOperationPhaseV1::Adopted
    );
    let remaining_funding_attempts = if terminal {
        0
    } else {
        MAX_FUNDING_ATTEMPTS
            .checked_sub(operation.funding_attempts.len())
            .ok_or(RetainedRootRepairProcedureError::FundingAttemptBoundExceeded)?
    };
    let actual_cycles = (operation.phase == RetainedRootRepairOperationPhaseV1::FundingRequired)
        .then_some(operation.final_actual_cycles)
        .flatten();
    let exact_plan = actual_cycles
        .map(|actual| {
            funding_plan(
                actual,
                receipt.required_pool_cycles,
                receipt.top_up_fee_cycles,
                receipt.top_up_margin_cycles,
            )
        })
        .transpose()?;
    let next = (remaining_funding_attempts > 0)
        .then_some(exact_plan)
        .flatten();
    let worst_case_attempt = receipt
        .required_pool_cycles
        .checked_add(receipt.top_up_margin_cycles)
        .and_then(|cycles| cycles.checked_add(receipt.top_up_fee_cycles))
        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?;
    let cumulative_remaining_authority_cycles = match next {
        Some(next) => {
            let later_attempts = remaining_funding_attempts.saturating_sub(1);
            next.maximum_operator_debit_cycles
                .checked_add(
                    worst_case_attempt
                        .checked_mul(later_attempts as u128)
                        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?,
                )
                .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?
        }
        None => worst_case_attempt
            .checked_mul(remaining_funding_attempts as u128)
            .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?,
    };
    let authorization_digest = next
        .map(|plan| {
            funding_authorization_digest(
                operation,
                receipt,
                plan,
                remaining_funding_attempts,
                cumulative_remaining_authority_cycles,
            )
        })
        .transpose()?;
    Ok(RetainedRootRepairFundingExposureV1 {
        repair_operation_id: hex_digest(operation.repair_operation_id),
        fleet_subnet_root: operation.fleet_subnet_root.to_text(),
        pool_canister: operation.pool_canister.to_text(),
        operator: receipt.installation_controller.to_text(),
        phase: operation_phase_label(operation.phase).to_string(),
        actual_cycles,
        required_cycles: receipt.required_pool_cycles,
        deficit_cycles: exact_plan.map(|plan| plan.deficit_cycles),
        next_requested_cycles: next.map(|plan| plan.requested_cycles),
        next_maximum_operator_debit_cycles: next.map(|plan| plan.maximum_operator_debit_cycles),
        cumulative_remaining_authority_cycles,
        completed_funding_attempts: operation.funding_attempts.len(),
        remaining_funding_attempts,
        authorization_digest,
        operator_available_cycles: None,
        operator_balance_sufficient: None,
    })
}

fn funding_authorization_digest(
    operation: &RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
    plan: RetainedRootRepairFundingPlan,
    remaining_funding_attempts: usize,
    cumulative_remaining_authority_cycles: u128,
) -> Result<String, RetainedRootRepairProcedureError> {
    let authority = RetainedRootRepairFundingAuthorizationV1 {
        schema_version: 1,
        repair_operation_id: operation.repair_operation_id,
        fleet_subnet_root: operation.fleet_subnet_root,
        pool_canister: operation.pool_canister,
        operator: receipt.installation_controller,
        actual_cycles: plan.actual_cycles,
        required_cycles: plan.required_cycles,
        deficit_cycles: plan.deficit_cycles,
        requested_cycles: plan.requested_cycles,
        maximum_operator_debit_cycles: plan.maximum_operator_debit_cycles,
        cumulative_remaining_authority_cycles,
        completed_funding_attempts: operation.funding_attempts.len(),
        remaining_funding_attempts,
    };
    let bytes = serde_json::to_vec(&authority).map_err(|error| {
        invalid(
            std::path::Path::new(REPAIR_OPERATION_FILE),
            error.to_string(),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"canic.retained-root-repair.funding-authorization.v1\0");
    hasher.update(bytes);
    Ok(hex_digest(hasher.finalize().into()))
}

const fn operation_phase_label(phase: RetainedRootRepairOperationPhaseV1) -> &'static str {
    match phase {
        RetainedRootRepairOperationPhaseV1::Planned => "planned",
        RetainedRootRepairOperationPhaseV1::UpgradeInFlight => "upgrade_in_flight",
        RetainedRootRepairOperationPhaseV1::UpgradeVerified => "upgrade_verified",
        RetainedRootRepairOperationPhaseV1::TopUpInFlight => "top_up_in_flight",
        RetainedRootRepairOperationPhaseV1::ReinspectionInFlight => "reinspection_in_flight",
        RetainedRootRepairOperationPhaseV1::FundingRequired => "funding_required",
        RetainedRootRepairOperationPhaseV1::AssetReady => "asset_ready",
        RetainedRootRepairOperationPhaseV1::Adopted => "adopted",
    }
}

fn repairable_failure_reason(asset: &CanisterPoolAsset) -> String {
    match &asset.status {
        CanisterPoolAssetStatus::Failed { reason } => format!("ResetFailed(reason={reason})"),
        CanisterPoolAssetStatus::Ready => "Ready below the retained requirement".to_string(),
        CanisterPoolAssetStatus::PendingReset => {
            "PendingReset below the retained requirement".to_string()
        }
        _ => "an unexpected repairable state".to_string(),
    }
}

fn funding_plan(
    actual_cycles: u128,
    required_cycles: u128,
    fee_cycles: u128,
    margin_cycles: u128,
) -> Result<RetainedRootRepairFundingPlan, RetainedRootRepairProcedureError> {
    let deficit_cycles = required_cycles.saturating_sub(actual_cycles);
    let requested_cycles = deficit_cycles
        .checked_add(margin_cycles)
        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?;
    let maximum_operator_debit_cycles = requested_cycles
        .checked_add(fee_cycles)
        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?;
    Ok(RetainedRootRepairFundingPlan {
        actual_cycles,
        required_cycles,
        deficit_cycles,
        fee_cycles,
        margin_cycles,
        requested_cycles,
        maximum_operator_debit_cycles,
    })
}

fn funding_observation(
    attempt: &RetainedRootRepairFundingAttemptV1,
    actual_cycles_after: u128,
    operator_cycles_after: u128,
    command_succeeded: bool,
) -> Result<RetainedRootRepairFundingObservation, RetainedRootRepairProcedureError> {
    let operator_debit_cycles = attempt
        .operator_cycles_before
        .checked_sub(operator_cycles_after)
        .ok_or(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)?;
    let asset_credit_cycles = actual_cycles_after
        .checked_sub(attempt.actual_cycles_before)
        .ok_or(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)?;
    if operator_debit_cycles != attempt.maximum_operator_debit_cycles
        || (!command_succeeded && asset_credit_cycles == 0)
    {
        return Err(RetainedRootRepairProcedureError::TopUpOutcomeUnknown);
    }
    let observed_fee_and_burn_cycles = operator_debit_cycles
        .checked_sub(asset_credit_cycles)
        .ok_or(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)?;
    if asset_credit_cycles > attempt.requested_cycles
        || observed_fee_and_burn_cycles < attempt.fee_cycles
    {
        return Err(RetainedRootRepairProcedureError::TopUpOutcomeUnknown);
    }
    Ok(RetainedRootRepairFundingObservation {
        operator_debit_cycles,
        asset_credit_cycles,
        observed_fee_and_burn_cycles,
    })
}

fn create_or_load_operation(
    path: &std::path::Path,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<RetainedRootRepairOperationV1, RetainedRootRepairProcedureError> {
    let expected = planned_operation(receipt);
    if let Some(current) = load_optional_operation(path)? {
        validate_operation(path, &current, receipt)?;
        return Ok(current);
    }
    let bytes = encode_operation(path, &expected)?;
    match create_new_bytes(path, &bytes) {
        Ok(()) => Ok(expected),
        Err(source) if source.kind() == io::ErrorKind::AlreadyExists => {
            let current = load_optional_operation(path)?.ok_or_else(|| {
                RetainedRootRepairProcedureError::ConflictingAuthority {
                    path: path.to_path_buf(),
                }
            })?;
            validate_operation(path, &current, receipt)?;
            Ok(current)
        }
        Err(source) => Err(RetainedRootRepairProcedureError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

const fn planned_operation(
    receipt: &RetainedRootRepairAuthorityV1,
) -> RetainedRootRepairOperationV1 {
    RetainedRootRepairOperationV1 {
        schema_version: REPAIR_OPERATION_SCHEMA_VERSION,
        repair_operation_id: receipt.repair_operation_id,
        fleet_subnet_root: receipt.fleet_subnet_root,
        pool_canister: receipt.pool_canister,
        upgrade_predecessor_module_sha256: receipt.upgrade_predecessor_module_sha256,
        successor_module_sha256: receipt.successor_module_sha256,
        required_pool_cycles: receipt.required_pool_cycles,
        top_up_fee_cycles: receipt.top_up_fee_cycles,
        top_up_margin_cycles: receipt.top_up_margin_cycles,
        phase: RetainedRootRepairOperationPhaseV1::Planned,
        upgrade_command_receipt: None,
        funding_attempts: Vec::new(),
        final_actual_cycles: None,
    }
}

fn validate_operation(
    path: &std::path::Path,
    operation: &RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairProcedureError> {
    let authority_matches = operation.schema_version == REPAIR_OPERATION_SCHEMA_VERSION
        && operation.repair_operation_id == receipt.repair_operation_id
        && operation.fleet_subnet_root == receipt.fleet_subnet_root
        && operation.pool_canister == receipt.pool_canister
        && operation.upgrade_predecessor_module_sha256 == receipt.upgrade_predecessor_module_sha256
        && operation.successor_module_sha256 == receipt.successor_module_sha256
        && operation.required_pool_cycles == receipt.required_pool_cycles
        && operation.top_up_fee_cycles == receipt.top_up_fee_cycles
        && operation.top_up_margin_cycles == receipt.top_up_margin_cycles;
    if !authority_matches || operation.funding_attempts.len() > MAX_FUNDING_ATTEMPTS {
        return Err(RetainedRootRepairProcedureError::ConflictingAuthority {
            path: path.to_path_buf(),
        });
    }
    if matches!(
        operation.phase,
        RetainedRootRepairOperationPhaseV1::AssetReady
            | RetainedRootRepairOperationPhaseV1::Adopted
    ) && operation
        .final_actual_cycles
        .is_none_or(|actual| actual < operation.required_pool_cycles)
    {
        return Err(invalid(
            path,
            "terminal repair operation omits exact adequate pool balance",
        ));
    }
    if operation.phase == RetainedRootRepairOperationPhaseV1::FundingRequired
        && operation
            .final_actual_cycles
            .is_none_or(|actual| actual >= operation.required_pool_cycles)
    {
        return Err(invalid(
            path,
            "funding-required repair operation omits its exact below-threshold balance",
        ));
    }
    validate_funding_attempts(path, operation, receipt)?;
    Ok(())
}

fn validate_funding_attempts(
    path: &std::path::Path,
    operation: &RetainedRootRepairOperationV1,
    receipt: &RetainedRootRepairAuthorityV1,
) -> Result<(), RetainedRootRepairProcedureError> {
    let mut has_incomplete_attempt = false;
    for (index, attempt) in operation.funding_attempts.iter().enumerate() {
        if usize::from(attempt.sequence) != index + 1
            || attempt.required_cycles != receipt.required_pool_cycles
            || attempt.fee_cycles != receipt.top_up_fee_cycles
            || attempt.margin_cycles != receipt.top_up_margin_cycles
            || attempt
                .command_receipt
                .as_ref()
                .is_some_and(|receipt| receipt.len() > MAX_COMMAND_RECEIPT_BYTES)
        {
            return Err(invalid(
                path,
                "funding attempt differs from exact repair authority",
            ));
        }
        let expected = funding_plan(
            attempt.actual_cycles_before,
            attempt.required_cycles,
            attempt.fee_cycles,
            attempt.margin_cycles,
        )?;
        if expected.deficit_cycles != attempt.deficit_cycles
            || expected.requested_cycles != attempt.requested_cycles
            || expected.maximum_operator_debit_cycles != attempt.maximum_operator_debit_cycles
        {
            return Err(invalid(path, "funding attempt arithmetic is inconsistent"));
        }
        let observations = [
            attempt.actual_cycles_after,
            attempt.operator_cycles_after,
            attempt.operator_debit_cycles,
            attempt.asset_credit_cycles,
            attempt.observed_fee_and_burn_cycles,
            attempt.retained_margin_cycles,
        ];
        let observation_is_complete = observations.iter().all(Option::is_some);
        if observations.iter().any(Option::is_some) != observation_is_complete {
            return Err(invalid(
                path,
                "funding observation is only partially retained",
            ));
        }
        if !observation_is_complete {
            if index + 1 != operation.funding_attempts.len() {
                return Err(invalid(
                    path,
                    "only the final funding attempt may retain an incomplete observation",
                ));
            }
            has_incomplete_attempt = true;
        }
        if observation_is_complete {
            let actual_after = attempt
                .actual_cycles_after
                .expect("complete observation has actual balance");
            let operator_after = attempt
                .operator_cycles_after
                .expect("complete observation has operator balance");
            let observation = funding_observation(attempt, actual_after, operator_after, true)?;
            if Some(observation.operator_debit_cycles) != attempt.operator_debit_cycles
                || Some(observation.asset_credit_cycles) != attempt.asset_credit_cycles
                || Some(observation.observed_fee_and_burn_cycles)
                    != attempt.observed_fee_and_burn_cycles
                || attempt.retained_margin_cycles
                    != Some(actual_after.saturating_sub(receipt.required_pool_cycles))
            {
                return Err(invalid(
                    path,
                    "funding observation arithmetic is inconsistent",
                ));
            }
        }
    }
    if !funding_phase_accepts_observation(operation.phase, has_incomplete_attempt) {
        return Err(invalid(
            path,
            "repair phase disagrees with its retained funding observation",
        ));
    }
    Ok(())
}

const fn funding_phase_accepts_observation(
    phase: RetainedRootRepairOperationPhaseV1,
    has_incomplete_attempt: bool,
) -> bool {
    match phase {
        RetainedRootRepairOperationPhaseV1::TopUpInFlight => has_incomplete_attempt,
        RetainedRootRepairOperationPhaseV1::ReinspectionInFlight => true,
        RetainedRootRepairOperationPhaseV1::Planned
        | RetainedRootRepairOperationPhaseV1::UpgradeInFlight
        | RetainedRootRepairOperationPhaseV1::UpgradeVerified
        | RetainedRootRepairOperationPhaseV1::FundingRequired
        | RetainedRootRepairOperationPhaseV1::AssetReady
        | RetainedRootRepairOperationPhaseV1::Adopted => !has_incomplete_attempt,
    }
}

fn load_optional_operation(
    path: &std::path::Path,
) -> Result<Option<RetainedRootRepairOperationV1>, RetainedRootRepairProcedureError> {
    let bytes = match read_optional_bounded_regular_bytes(path, MAX_REPAIR_OPERATION_BYTES) {
        Ok(bytes) => bytes,
        Err(BoundedRegularFileReadError::TooLarge) => {
            return Err(invalid(path, "repair operation exceeds its byte bound"));
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::NotRegular)) => {
            return Err(RetainedRootRepairProcedureError::UnsafeFile {
                path: path.to_path_buf(),
            });
        }
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::Io(source))) => {
            return Err(RetainedRootRepairProcedureError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
        #[cfg(not(unix))]
        Err(BoundedRegularFileReadError::Read(RegularFileReadError::UnsupportedPlatform)) => {
            return Err(RetainedRootRepairProcedureError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair operation reads are unsupported",
                ),
            });
        }
    };
    let Some(bytes) = bytes else {
        return Ok(None);
    };
    let operation = serde_json::from_slice::<RetainedRootRepairOperationV1>(&bytes)
        .map_err(|error| invalid(path, error.to_string()))?;
    if encode_operation(path, &operation)? != bytes {
        return Err(invalid(path, "repair operation bytes are not canonical"));
    }
    Ok(Some(operation))
}

fn replace_operation(
    path: &std::path::Path,
    operation: &RetainedRootRepairOperationV1,
) -> Result<(), RetainedRootRepairProcedureError> {
    let bytes = encode_operation(path, operation)?;
    replace_bytes_exact(path, &bytes).map_err(|error| replace_error(path, error))?;
    let durable =
        load_optional_operation(path)?.ok_or_else(|| invalid(path, "operation missing"))?;
    if durable != *operation {
        return Err(invalid(
            path,
            "operation replacement did not retain exact bytes",
        ));
    }
    Ok(())
}

fn encode_operation(
    path: &std::path::Path,
    operation: &RetainedRootRepairOperationV1,
) -> Result<Vec<u8>, RetainedRootRepairProcedureError> {
    encode_canonical_json(
        operation,
        CanonicalJsonStyle::Compact,
        MAX_REPAIR_OPERATION_BYTES,
    )
    .map_err(|error| match error {
        CanonicalJsonEncodeError::Serialization(error) => invalid(path, error.to_string()),
        CanonicalJsonEncodeError::TooLarge => {
            invalid(path, "repair operation exceeds its byte bound")
        }
    })
}

fn lock_operation(
    path: &std::path::Path,
) -> Result<std::fs::File, RetainedRootRepairProcedureError> {
    match lock_regular_file_with_parents(path) {
        Ok(lock) => Ok(lock),
        Err(RegularFileLockError::NotRegular) => {
            Err(RetainedRootRepairProcedureError::UnsafeLock {
                path: path.to_path_buf(),
            })
        }
        Err(RegularFileLockError::Io(source)) => Err(RetainedRootRepairProcedureError::Io {
            path: path.to_path_buf(),
            source,
        }),
        #[cfg(windows)]
        Err(RegularFileLockError::UnsupportedPlatform) => {
            Err(RetainedRootRepairProcedureError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair operation locking is unsupported",
                ),
            })
        }
    }
}

fn bounded_command_receipt(
    receipt: String,
    too_large: RetainedRootRepairProcedureError,
) -> Result<String, RetainedRootRepairProcedureError> {
    if receipt.len() > MAX_COMMAND_RECEIPT_BYTES {
        Err(too_large)
    } else {
        Ok(receipt)
    }
}

#[derive(Serialize)]
struct RetainedRootRepairReconciliationOutput {
    repair_operation_id: String,
    fleet_subnet_root: Principal,
    pool_canister: Principal,
    phase: RetainedRootRepairOperationPhaseV1,
    actual_cycles: u128,
    required_cycles: u128,
    deficit_cycles: u128,
    fee_cycles: u128,
    margin_cycles: u128,
    requested_cycles: u128,
    maximum_operator_debit_cycles: u128,
    operator_debit_cycles: u128,
    asset_credit_cycles: u128,
    observed_fee_and_burn_cycles: u128,
    retained_margin_cycles: u128,
    funding_attempts: usize,
}

fn print_reconciliation(
    operation: &RetainedRootRepairOperationV1,
) -> Result<(), RetainedRootRepairProcedureError> {
    let actual_cycles = operation.final_actual_cycles.ok_or_else(|| {
        invalid(
            std::path::Path::new(REPAIR_OPERATION_FILE),
            "final balance missing",
        )
    })?;
    let original_actual = operation
        .funding_attempts
        .first()
        .map_or(actual_cycles, |attempt| attempt.actual_cycles_before);
    let requested_cycles = operation
        .funding_attempts
        .iter()
        .try_fold(0_u128, |total, attempt| {
            total.checked_add(attempt.requested_cycles)
        })
        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?;
    let operator_debit_cycles = sum_observed(&operation.funding_attempts, |attempt| {
        attempt.operator_debit_cycles
    })?;
    let maximum_operator_debit_cycles = operation
        .funding_attempts
        .iter()
        .try_fold(0_u128, |total, attempt| {
            total.checked_add(attempt.maximum_operator_debit_cycles)
        })
        .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)?;
    let asset_credit_cycles = sum_observed(&operation.funding_attempts, |attempt| {
        attempt.asset_credit_cycles
    })?;
    let observed_fee_and_burn_cycles = sum_observed(&operation.funding_attempts, |attempt| {
        attempt.observed_fee_and_burn_cycles
    })?;
    let output = RetainedRootRepairReconciliationOutput {
        repair_operation_id: hex_digest(operation.repair_operation_id),
        fleet_subnet_root: operation.fleet_subnet_root,
        pool_canister: operation.pool_canister,
        phase: operation.phase,
        actual_cycles,
        required_cycles: operation.required_pool_cycles,
        deficit_cycles: operation
            .required_pool_cycles
            .saturating_sub(original_actual),
        fee_cycles: operation.top_up_fee_cycles,
        margin_cycles: operation.top_up_margin_cycles,
        requested_cycles,
        maximum_operator_debit_cycles,
        operator_debit_cycles,
        asset_credit_cycles,
        observed_fee_and_burn_cycles,
        retained_margin_cycles: actual_cycles.saturating_sub(operation.required_pool_cycles),
        funding_attempts: operation.funding_attempts.len(),
    };
    let encoded = serde_json::to_string(&output).map_err(|error| {
        invalid(
            std::path::Path::new(REPAIR_OPERATION_FILE),
            error.to_string(),
        )
    })?;
    println!("Retained Root repair reconciliation: {encoded}");
    Ok(())
}

fn sum_observed(
    attempts: &[RetainedRootRepairFundingAttemptV1],
    select: impl Fn(&RetainedRootRepairFundingAttemptV1) -> Option<u128>,
) -> Result<u128, RetainedRootRepairProcedureError> {
    attempts.iter().try_fold(0_u128, |total, attempt| {
        total
            .checked_add(select(attempt).unwrap_or_default())
            .ok_or(RetainedRootRepairProcedureError::ArithmeticOverflow)
    })
}

fn hex_digest(digest: [u8; 32]) -> String {
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

fn resolved_sibling(path: &std::path::Path, file: &str) -> PathBuf {
    path.with_file_name(file)
}

fn replace_error(
    path: &std::path::Path,
    error: ExactReplaceError,
) -> RetainedRootRepairProcedureError {
    match error {
        ExactReplaceError::Read(RegularFileReadError::NotRegular) => {
            RetainedRootRepairProcedureError::UnsafeFile {
                path: path.to_path_buf(),
            }
        }
        ExactReplaceError::Read(RegularFileReadError::Io(source))
        | ExactReplaceError::Write(source) => RetainedRootRepairProcedureError::Io {
            path: path.to_path_buf(),
            source,
        },
        #[cfg(not(unix))]
        ExactReplaceError::Read(RegularFileReadError::UnsupportedPlatform) => {
            RetainedRootRepairProcedureError::Io {
                path: path.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::Unsupported,
                    "retained Root repair operation replacement is unsupported",
                ),
            }
        }
    }
}

fn invalid(path: &std::path::Path, reason: impl Into<String>) -> RetainedRootRepairProcedureError {
    RetainedRootRepairProcedureError::InvalidDocument {
        path: path.to_path_buf(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funding_attempt() -> RetainedRootRepairFundingAttemptV1 {
        RetainedRootRepairFundingAttemptV1 {
            sequence: 1,
            actual_cycles_before: 4_999_546_217_226,
            required_cycles: 5_000_000_000_000,
            deficit_cycles: 453_782_774,
            fee_cycles: 100_000_000,
            margin_cycles: 100_000_000,
            requested_cycles: 553_782_774,
            maximum_operator_debit_cycles: 653_782_774,
            operator_cycles_before: 1_000_000_000,
            command_receipt: None,
            actual_cycles_after: None,
            operator_cycles_after: None,
            operator_debit_cycles: None,
            asset_credit_cycles: None,
            observed_fee_and_burn_cycles: None,
            retained_margin_cycles: None,
        }
    }

    #[test]
    fn funding_plan_exposes_exact_raw_deficit_fee_margin_and_operator_debit() {
        let plan = funding_plan(
            4_999_546_217_226,
            5_000_000_000_000,
            100_000_000,
            100_000_000,
        )
        .expect("exact funding plan");

        assert_eq!(plan.actual_cycles, 4_999_546_217_226);
        assert_eq!(plan.required_cycles, 5_000_000_000_000);
        assert_eq!(plan.deficit_cycles, 453_782_774);
        assert_eq!(plan.fee_cycles, 100_000_000);
        assert_eq!(plan.margin_cycles, 100_000_000);
        assert_eq!(plan.requested_cycles, 553_782_774);
        assert_eq!(plan.maximum_operator_debit_cycles, 653_782_774);
    }

    #[test]
    fn funding_plan_rejects_overflow_before_an_effect() {
        assert!(matches!(
            funding_plan(0, u128::MAX, 1, 0),
            Err(RetainedRootRepairProcedureError::ArithmeticOverflow)
        ));
    }

    #[test]
    fn funding_observation_reconciles_exact_credit_and_separate_fee() {
        let observation =
            funding_observation(&funding_attempt(), 5_000_100_000_000, 346_217_226, true)
                .expect("exact top-up observation");

        assert_eq!(observation.operator_debit_cycles, 653_782_774);
        assert_eq!(observation.asset_credit_cycles, 553_782_774);
        assert_eq!(observation.observed_fee_and_burn_cycles, 100_000_000);
    }

    #[test]
    fn funding_observation_recovers_response_loss_from_balances() {
        let observation =
            funding_observation(&funding_attempt(), 5_000_050_000_000, 346_217_226, false)
                .expect("paid effect is visible despite response loss");

        assert_eq!(observation.asset_credit_cycles, 503_782_774);
        assert_eq!(observation.observed_fee_and_burn_cycles, 150_000_000);
    }

    #[test]
    fn funding_observation_rejects_absent_or_conflicting_effects() {
        let attempt = funding_attempt();
        assert!(matches!(
            funding_observation(
                &attempt,
                attempt.actual_cycles_before,
                attempt.operator_cycles_before,
                false,
            ),
            Err(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)
        ));
        assert!(matches!(
            funding_observation(
                &attempt,
                attempt.actual_cycles_before + attempt.requested_cycles + 1,
                attempt.operator_cycles_before - attempt.maximum_operator_debit_cycles,
                true,
            ),
            Err(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)
        ));
        assert!(matches!(
            funding_observation(
                &attempt,
                attempt.actual_cycles_before + attempt.requested_cycles,
                attempt.operator_cycles_before - attempt.maximum_operator_debit_cycles + 1,
                true,
            ),
            Err(RetainedRootRepairProcedureError::TopUpOutcomeUnknown)
        ));
    }

    #[test]
    fn funding_phase_accepts_zero_attempt_reinspection_without_weakening_top_up() {
        assert!(funding_phase_accepts_observation(
            RetainedRootRepairOperationPhaseV1::ReinspectionInFlight,
            false,
        ));
        assert!(funding_phase_accepts_observation(
            RetainedRootRepairOperationPhaseV1::ReinspectionInFlight,
            true,
        ));
        assert!(funding_phase_accepts_observation(
            RetainedRootRepairOperationPhaseV1::TopUpInFlight,
            true,
        ));
        assert!(!funding_phase_accepts_observation(
            RetainedRootRepairOperationPhaseV1::TopUpInFlight,
            false,
        ));
        assert!(!funding_phase_accepts_observation(
            RetainedRootRepairOperationPhaseV1::AssetReady,
            true,
        ));
    }
}
