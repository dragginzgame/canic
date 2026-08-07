//! Module: ops::component_provisioning_receipt
//!
//! Responsibility: hash exact root-issued Component provisioning receipts.
//! Does not own: root persistence, receipt transport, or Coordinator orchestration.
//! Boundary: root and Coordinator code share this one canonical receipt authority.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        component_provisioning::{
            FleetSubnetRootProvisioningBatch, RootComponentProvisioningResult,
        },
        fleet_registry::FleetRegistryVersion,
    },
    ids::{ComponentDeploymentConfigurationDigest, FleetSubnetRootBinding},
};
use candid::CandidType;
use sha2::{Digest, Sha256};

const ACCEPTANCE_RECEIPT_DOMAIN: &[u8] = b"canic/root-component-provisioning-acceptance-receipt/v1";
const PROVISIONED_RECEIPT_DOMAIN: &[u8] =
    b"canic/root-component-provisioning-provisioned-receipt/v1";

/// Exact immutable fields covered by one root's initial `Accepted` receipt.
#[derive(CandidType)]
pub struct RootComponentProvisioningAcceptanceReceiptAuthority<'a> {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: &'a FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub batch: &'a FleetSubnetRootProvisioningBatch,
    pub placement_count: u32,
    pub component_count: u32,
    pub accepted_at_ns: u64,
}

/// Exact immutable fields covered by one root's terminal `Provisioned` receipt.
#[derive(CandidType)]
pub struct RootComponentProvisioningProvisionedReceiptAuthority<'a> {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub fleet_registry: &'a FleetRegistryVersion,
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub root: &'a FleetSubnetRootBinding,
    pub result: &'a RootComponentProvisioningResult,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: u64,
}

/// Canonical hashing boundary shared by root receipt production and Coordinator verification.
pub struct RootComponentProvisioningReceiptOps;

impl RootComponentProvisioningReceiptOps {
    /// Hash one exact root acceptance receipt with its frozen domain.
    pub fn acceptance_content_hash(
        authority: RootComponentProvisioningAcceptanceReceiptAuthority<'_>,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(
            ACCEPTANCE_RECEIPT_DOMAIN,
            authority,
            "root Component provisioning acceptance receipt",
        )
    }

    /// Hash one exact terminal root provisioning receipt with its frozen domain.
    pub fn provisioned_content_hash(
        authority: RootComponentProvisioningProvisionedReceiptAuthority<'_>,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(
            PROVISIONED_RECEIPT_DOMAIN,
            authority,
            "root Component provisioning receipt",
        )
    }
}

fn receipt_content_hash(
    domain: &[u8],
    authority: impl CandidType,
    label: &str,
) -> Result<[u8; 32], InternalError> {
    let bytes = candid::encode_one(authority).map_err(|error| {
        InternalError::invariant(
            InternalErrorOrigin::Ops,
            format!("could not encode {label}: {error}"),
        )
    })?;
    let byte_count = u64::try_from(bytes.len()).map_err(|_| {
        InternalError::resource_exhausted(format!("{label} exceeds the canonical byte-count range"))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(byte_count.to_be_bytes());
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}
