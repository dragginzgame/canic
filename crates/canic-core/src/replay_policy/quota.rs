//! Module: replay_policy::quota
//!
//! Responsibility: name quota and cycle-reserve policy labels used by manifests.
//! Does not own: cost guard enforcement, replay guard execution, or storage.
//! Boundary: label constants referenced by replay-policy manifest rows.

use super::types::{ReplayCycleReservePolicyLabel, ReplayQuotaPolicyLabel};

pub(super) const ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("root_canister_signature_prepare.quota.v1");
pub(super) const ROOT_CHAIN_KEY_SIGNING_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("root_chain_key_signing.quota.v1");
pub(super) const ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("issuer_canister_signature_prepare.quota.v1");
pub(super) const DEPLOYMENT_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("deployment.quota.v1");
pub(super) const DEPLOYMENT_RESERVE_V1: ReplayCycleReservePolicyLabel =
    ReplayCycleReservePolicyLabel::new("deployment.cycle_reserve.v1");
pub(super) const VALUE_TRANSFER_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("value_transfer.quota.v1");
pub(super) const VALUE_TRANSFER_RESERVE_V1: ReplayCycleReservePolicyLabel =
    ReplayCycleReservePolicyLabel::new("value_transfer.cycle_reserve.v1");
pub(super) const DURABLE_PUBLISH_QUOTA_V1: ReplayQuotaPolicyLabel =
    ReplayQuotaPolicyLabel::new("durable_publish.quota.v1");
pub(super) const DURABLE_PUBLISH_RESERVE_V1: ReplayCycleReservePolicyLabel =
    ReplayCycleReservePolicyLabel::new("durable_publish.cycle_reserve.v1");
