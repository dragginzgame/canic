//! Module: replay_policy::quota
//!
//! Responsibility: name quota and cycle-reserve policy labels used by manifests.
//! Does not own: cost guard enforcement, replay guard execution, or storage.
//! Boundary: label constants referenced by replay-policy manifest rows.

pub(super) const ROOT_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: &str =
    "root_canister_signature_prepare.quota.v1";
pub(super) const ISSUER_CANISTER_SIGNATURE_PREPARE_QUOTA_V1: &str =
    "issuer_canister_signature_prepare.quota.v1";
pub(super) const DEPLOYMENT_QUOTA_V1: &str = "deployment.quota.v1";
pub(super) const DEPLOYMENT_RESERVE_V1: &str = "deployment.cycle_reserve.v1";
pub(super) const VALUE_TRANSFER_QUOTA_V1: &str = "value_transfer.quota.v1";
pub(super) const VALUE_TRANSFER_RESERVE_V1: &str = "value_transfer.cycle_reserve.v1";
pub(super) const DURABLE_PUBLISH_QUOTA_V1: &str = "durable_publish.quota.v1";
pub(super) const DURABLE_PUBLISH_RESERVE_V1: &str = "durable_publish.cycle_reserve.v1";
