//! Module: ops::component_provisioning_receipt
//!
//! Responsibility: hash exact root-issued Component provisioning receipts.
//! Does not own: root persistence, receipt transport, or Coordinator orchestration.
//! Boundary: root and Coordinator code share this one canonical receipt authority.

use crate::{
    InternalError, InternalErrorOrigin,
    dto::{
        component_provisioning::{
            ComponentGroupDirectory, FleetSubnetRootProvisioningBatch,
            RootComponentProvisioningResult, RootComponentPublicationEvidence,
            RootProvisionedGroupPlacement,
        },
        fleet_registry::{FleetDirectorySnapshot, FleetRegistryVersion},
    },
    ids::{ComponentDeploymentConfigurationDigest, FleetSubnetRootBinding},
};
use candid::CandidType;
use sha2::{Digest, Sha256};

const ACCEPTANCE_RECEIPT_DOMAIN: &[u8] = b"canic/root-component-provisioning-acceptance-receipt/v1";
const PROVISIONED_RECEIPT_DOMAIN: &[u8] =
    b"canic/root-component-provisioning-provisioned-receipt/v1";
const PUBLISHED_RECEIPT_DOMAIN: &[u8] = b"canic/root-component-provisioning-published-receipt/v1";
const FLEET_DIRECTORY_DOMAIN: &[u8] = b"canic/fleet-directory/v1";
const COMPONENT_GROUP_DIRECTORY_DOMAIN: &[u8] = b"canic/component-group-directory/v1";
const GROUP_PLACEMENT_RECEIPT_DOMAIN: &[u8] =
    b"canic/root-component-provisioning-group-placement-receipt/v1";

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

/// Exact immutable fields covered by one root's terminal `Published` receipt.
#[derive(CandidType)]
pub struct RootComponentProvisioningPublishedReceiptAuthority<'a> {
    pub operation_id: [u8; 32],
    pub plan_hash: [u8; 32],
    pub configuration_digest: ComponentDeploymentConfigurationDigest,
    pub root: &'a FleetSubnetRootBinding,
    pub result: &'a RootComponentProvisioningResult,
    pub publication: &'a RootComponentPublicationEvidence,
    pub accepted_at_ns: u64,
    pub provisioned_at_ns: u64,
    pub published_at_ns: u64,
}

#[derive(CandidType)]
struct RootComponentGroupPlacementReceiptAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    root: &'a FleetSubnetRootBinding,
    placement: &'a RootProvisionedGroupPlacement,
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

    /// Hash one exact terminal root publication receipt with its frozen domain.
    pub fn published_content_hash(
        authority: RootComponentProvisioningPublishedReceiptAuthority<'_>,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(
            PUBLISHED_RECEIPT_DOMAIN,
            authority,
            "root Component publication receipt",
        )
    }

    /// Hash one exact Fleet Directory projection.
    pub fn fleet_directory_content_hash(
        directory: &FleetDirectorySnapshot,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(FLEET_DIRECTORY_DOMAIN, directory, "Fleet Directory")
    }

    /// Hash one exact Component Group Directory projection.
    pub fn component_group_directory_content_hash(
        directory: &ComponentGroupDirectory,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(
            COMPONENT_GROUP_DIRECTORY_DOMAIN,
            directory,
            "Component Group Directory",
        )
    }

    /// Hash one root-local placement result for Component Group Directory provenance.
    pub fn group_placement_content_hash(
        operation_id: [u8; 32],
        plan_hash: [u8; 32],
        root: &FleetSubnetRootBinding,
        placement: &RootProvisionedGroupPlacement,
    ) -> Result<[u8; 32], InternalError> {
        receipt_content_hash(
            GROUP_PLACEMENT_RECEIPT_DOMAIN,
            RootComponentGroupPlacementReceiptAuthority {
                operation_id,
                plan_hash,
                root,
                placement,
            },
            "root Component Group placement receipt",
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
