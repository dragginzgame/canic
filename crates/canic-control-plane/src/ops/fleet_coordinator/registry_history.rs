//! Module: ops::fleet_coordinator::registry_history
//!
//! Responsibility: reconstruct and validate the Coordinator's canonical Registry history.
//! Does not own: durable Registry storage, endpoint authorization, or lifecycle effects.
//! Boundary: consumes the one Coordinator record and validates each retained transition in order.

use super::root_lifecycle::{
    FleetSubnetRootDrainingIdentity, FleetSubnetRootRemovalPublicationIdentity,
    draining_reservation_for_publication, validate_draining_publication_request,
    validate_removal_publication_request,
};
use super::{
    FleetComponentProvisioningOperation, FleetComponentProvisioningRecord,
    FleetCoordinatorRegistryRecord, FleetRegistry, FleetRegistryOps, FleetRegistryVersion,
    FleetServicePublicationReceiptRecord, FleetSubnetRootDrainingPublicationReceiptRecord,
    FleetSubnetRootDrainingPublicationResponse, FleetSubnetRootEntry,
    FleetSubnetRootRemovalPublicationReceiptRecord, FleetSubnetRootRemovalPublicationResponse,
    FleetSubnetRootStatus, InternalError, MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
    apply_admission_publication_to_registry,
    fleet_funding_policy_rotation_successor_policy_set_hash, receipt_invariant,
};
use candid::Principal;
use canic_core::ids::{ComponentTopologyDigest, FleetSubnetRootReleaseSet, SubnetId};

pub(super) fn validate_registry_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let history = canonical_registry_lifecycle_history(current)?;
    if history
        .last()
        .is_none_or(|point| point.registry != current.registry)
    {
        return Err(receipt_invariant(
            "Fleet Registry head differs from its canonical lifecycle history",
        ));
    }
    Ok(())
}

#[derive(Clone)]
pub(super) struct FleetRegistryHistoryPoint {
    pub(super) registry: FleetRegistry,
    pub(super) version: FleetRegistryVersion,
}

#[derive(Clone, Copy)]
enum FleetSubnetRootLifecycleReceipt<'a> {
    Draining(&'a FleetSubnetRootDrainingPublicationReceiptRecord),
    Removed(&'a FleetSubnetRootRemovalPublicationReceiptRecord),
}

impl FleetSubnetRootLifecycleReceipt<'_> {
    const fn revision(self) -> u64 {
        match self {
            Self::Draining(receipt) => receipt.response.version.revision,
            Self::Removed(receipt) => receipt.response.version.revision,
        }
    }

    const fn previous_revision(self) -> u64 {
        match self {
            Self::Draining(receipt) => receipt.response.previous_version.revision,
            Self::Removed(receipt) => receipt.response.previous_version.revision,
        }
    }
}

pub(super) fn canonical_registry_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<Vec<FleetRegistryHistoryPoint>, InternalError> {
    let joining = historical_joining_registry(current)?;
    let (mut historical_registry, mut history) = initial_lifecycle_history(current, joining)?;
    let funding = crate::storage::stable::fleet_coordinator::FleetCoordinatorFundingStore::export();
    let checkpoints = funding
        .current
        .as_ref()
        .map_or(&[][..], |funding| funding.rotation_history.as_slice());
    let mut checkpoint_index = 0_usize;
    let mut admission_index = 0_usize;
    apply_service_publication_receipts(
        current,
        checkpoints,
        &mut checkpoint_index,
        &mut admission_index,
        &mut historical_registry,
        &mut history,
    )?;
    for lifecycle in canonical_lifecycle_receipts(current)? {
        apply_registry_policy_publications_through(
            current,
            checkpoints,
            &mut checkpoint_index,
            &mut admission_index,
            lifecycle.previous_revision(),
            &mut historical_registry,
            &mut history,
        )?;
        apply_lifecycle_receipt(current, lifecycle, &mut historical_registry, &mut history)?;
    }
    apply_registry_policy_publications_through(
        current,
        checkpoints,
        &mut checkpoint_index,
        &mut admission_index,
        current.registry.revision,
        &mut historical_registry,
        &mut history,
    )?;
    append_funding_policy_rotation_head(current, &mut historical_registry, &mut history)?;
    validate_pending_admission_publication(current, admission_index, &historical_registry)?;
    Ok(history)
}

fn apply_registry_policy_publications_through(
    current: &FleetCoordinatorRegistryRecord,
    checkpoints: &[crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationCheckpointRecord],
    checkpoint_index: &mut usize,
    admission_index: &mut usize,
    target_revision: u64,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    loop {
        let funding_revision = checkpoints
            .get(*checkpoint_index)
            .map(|checkpoint| checkpoint.receipt.successor_registry.revision);
        let admission_revision = current
            .admission_publications
            .get(*admission_index)
            .map(|publication| publication.version.revision);
        let next_revision = match (funding_revision, admission_revision) {
            (Some(funding), Some(admission)) if funding == admission => {
                return Err(receipt_invariant(
                    "Registry policy publications reuse one revision",
                ));
            }
            (Some(funding), Some(admission)) => funding.min(admission),
            (Some(funding), None) => funding,
            (None, Some(admission)) => admission,
            (None, None) => break,
        };
        if next_revision > target_revision {
            break;
        }
        if funding_revision == Some(next_revision) {
            let checkpoint = checkpoints
                .get(*checkpoint_index)
                .ok_or_else(InternalError::invariant)?;
            apply_funding_policy_rotation_checkpoint(
                current,
                checkpoint,
                historical_registry,
                history,
            )?;
            *checkpoint_index = checkpoint_index
                .checked_add(1)
                .ok_or_else(InternalError::invariant)?;
        } else {
            let publication = current
                .admission_publications
                .get(*admission_index)
                .ok_or_else(InternalError::invariant)?;
            let next =
                apply_admission_publication_to_registry(current, historical_registry, publication)?;
            let version = publication.version.clone();
            *historical_registry = next.clone();
            history.push(FleetRegistryHistoryPoint {
                registry: next,
                version,
            });
            *admission_index = admission_index
                .checked_add(1)
                .ok_or_else(InternalError::invariant)?;
        }
    }
    Ok(())
}

fn validate_pending_admission_publication(
    current: &FleetCoordinatorRegistryRecord,
    admission_index: usize,
    historical_registry: &FleetRegistry,
) -> Result<(), InternalError> {
    let remaining = current
        .admission_publications
        .get(admission_index..)
        .ok_or_else(InternalError::invariant)?;
    if remaining.is_empty() {
        return Ok(());
    }
    if remaining.len() != 1 || historical_registry != &current.registry {
        return Err(receipt_invariant(
            "Fleet admission publication history has non-canonical pending entries",
        ));
    }
    apply_admission_publication_to_registry(current, historical_registry, &remaining[0])?;
    Ok(())
}

fn apply_funding_policy_rotation_checkpoint(
    current: &FleetCoordinatorRegistryRecord,
    checkpoint: &crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationCheckpointRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
    )?;
    if previous_version != checkpoint.receipt.predecessor_registry
        || checkpoint.roots.len() != historical_registry.fleet_subnet_roots.len()
    {
        return Err(receipt_invariant(
            "funding-policy rotation checkpoint source differs from canonical history",
        ));
    }
    let mut next_registry = historical_registry.clone();
    next_registry.revision = next_registry
        .revision
        .checked_add(1)
        .ok_or_else(InternalError::invariant)?;
    for root in &checkpoint.roots {
        let entry = next_registry
            .fleet_subnet_roots
            .iter_mut()
            .find(|entry| entry.fleet_subnet_root == root.fleet_subnet_root)
            .ok_or_else(|| {
                receipt_invariant("funding-policy rotation checkpoint Root is not canonical")
            })?;
        entry.funding = root.funding.clone();
    }
    let policy_set_hash = fleet_funding_policy_rotation_successor_policy_set_hash(
        &checkpoint.coordinator_policy,
        checkpoint
            .roots
            .iter()
            .map(|root| (root.fleet_subnet_root, &root.funding)),
    );
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    if version != checkpoint.receipt.successor_registry
        || policy_set_hash != checkpoint.receipt.successor_policy_set_hash
    {
        return Err(receipt_invariant(
            "funding-policy rotation checkpoint target differs from canonical history",
        ));
    }
    *historical_registry = next_registry.clone();
    history.push(FleetRegistryHistoryPoint {
        registry: next_registry,
        version,
    });
    Ok(())
}

fn append_funding_policy_rotation_head(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    if historical_registry == &current.registry {
        return Ok(());
    }
    if !registry_diff_is_funding_rotation(historical_registry, &current.registry) {
        return Err(receipt_invariant(
            "Fleet Registry head differs from lifecycle history outside funding policy",
        ));
    }
    let funding = crate::storage::stable::fleet_coordinator::FleetCoordinatorFundingStore::export()
        .current
        .ok_or_else(InternalError::unavailable)?;
    let current_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &current.registry,
    )?;
    let active_authorizes = funding.rotation_current.as_ref().is_some_and(|rotation| {
        if current.registry.revision
            != rotation.header.predecessor_registry.revision.saturating_add(1)
            || current.root_funding.as_ref()
                != Some(&rotation.header.proposed_coordinator_policy)
            || matches!(
                rotation.phase,
                crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::Staging
            )
        {
            return false;
        }
        let roots_match = rotation.roots.len() == current.registry.fleet_subnet_roots.len()
            && rotation.roots.iter().all(|root| {
                current.registry.fleet_subnet_roots.iter().any(|entry| {
                    entry.fleet_subnet_root == root.fleet_subnet_root
                        && entry.funding.root_funding == root.proposed_policy
                })
            });
        let successor_matches = match &rotation.phase {
            crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::ActivatingRoots {
                successor_registry,
                ..
            } => successor_registry.as_ref() == &current_version,
            crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::PreparingRoots { .. } => true,
            crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationPhaseRecord::Staging => false,
        };
        roots_match && successor_matches
    });
    let terminal_authorizes = funding.rotation_last.as_ref().is_some_and(|receipt| {
        let Some(coordinator_policy) = current.root_funding.as_ref() else {
            return false;
        };
        let policy_hash = fleet_funding_policy_rotation_successor_policy_set_hash(
            coordinator_policy,
            current
                .registry
                .fleet_subnet_roots
                .iter()
                .map(|root| (root.fleet_subnet_root, &root.funding)),
        );
        receipt.successor_generation == funding.policy_generation
            && receipt.successor_registry == current_version
            && receipt.successor_policy_set_hash == policy_hash
    });
    if !active_authorizes && !terminal_authorizes {
        return Err(receipt_invariant(
            "Fleet Registry funding-policy head lacks exact durable rotation authority",
        ));
    }
    *historical_registry = current.registry.clone();
    history.push(FleetRegistryHistoryPoint {
        registry: current.registry.clone(),
        version: current_version,
    });
    Ok(())
}

fn registry_diff_is_funding_rotation(historical: &FleetRegistry, current: &FleetRegistry) -> bool {
    if historical.authority != current.authority
        || historical.component_specs != current.component_specs
        || historical.services != current.services
        || historical.fleet_subnet_roots.len() != current.fleet_subnet_roots.len()
        || current.revision <= historical.revision
    {
        return false;
    }
    historical
        .fleet_subnet_roots
        .iter()
        .zip(&current.fleet_subnet_roots)
        .all(|(historical, current)| {
            let mut current = current.clone();
            current.funding = historical.funding.clone();
            &current == historical
        })
}

fn initial_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
    joining: FleetRegistry,
) -> Result<(FleetRegistry, Vec<FleetRegistryHistoryPoint>), InternalError> {
    let Some(receipt) = &current.registry_activation_receipt else {
        let has_lifecycle_receipts = !current.service_publication_receipts.is_empty()
            || !current.root_draining_publication_receipts.is_empty()
            || !current.root_removal_publication_receipts.is_empty();
        if has_lifecycle_receipts || current.registry != joining {
            return Err(receipt_invariant(
                "Fleet Registry contains transitioned roots without an activation receipt",
            ));
        }
        let version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &joining,
        )?;
        return Ok((
            joining.clone(),
            vec![FleetRegistryHistoryPoint {
                registry: joining,
                version,
            }],
        ));
    };
    if !current.root_snapshot_acknowledgements.is_empty() {
        return Err(receipt_invariant(
            "active Fleet Registry retains stale Joining acknowledgements",
        ));
    }
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &joining,
    )
    .map_err(|_| receipt_invariant("activation source version cannot be derived"))?;
    let historical_registry = FleetRegistryOps::compile_active(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &joining,
    )
    .map_err(|_| receipt_invariant("activation target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &historical_registry,
    )
    .map_err(|_| receipt_invariant("activation target version cannot be derived"))?;
    if receipt.request.expected_registry != previous_version
        || receipt.response.previous_version != previous_version
        || receipt.response.version != version
    {
        return Err(receipt_invariant(
            "Fleet Registry activation receipt differs from canonical history",
        ));
    }
    let history = vec![FleetRegistryHistoryPoint {
        registry: historical_registry.clone(),
        version,
    }];
    Ok((historical_registry, history))
}

pub(super) fn initial_active_registry(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistry, InternalError> {
    let joining = historical_joining_registry(current)?;
    let (active, _) = initial_lifecycle_history(current, joining)?;
    if current.registry_activation_receipt.is_none() {
        return Err(InternalError::conflict());
    }
    Ok(active)
}

pub(super) fn component_operation_source_registry(
    current: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<FleetRegistry, InternalError> {
    match record.plan.operation {
        FleetComponentProvisioningOperation::FreshInstall => initial_active_registry(current),
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            registry_snapshot_at_version(current, &record.plan.fleet_registry)
        }
    }
}

pub(super) fn registry_snapshot_at_version(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
) -> Result<FleetRegistry, InternalError> {
    canonical_registry_lifecycle_history(current)?
        .into_iter()
        .find(|point| &point.version == version)
        .map(|point| point.registry)
        .ok_or_else(|| {
            receipt_invariant("Fleet Component operation source Registry is absent from history")
        })
}

fn apply_service_publication_receipts(
    current: &FleetCoordinatorRegistryRecord,
    checkpoints: &[crate::storage::stable::fleet_coordinator::FleetFundingPolicyRotationCheckpointRecord],
    checkpoint_index: &mut usize,
    admission_index: &mut usize,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    for receipt in &current.service_publication_receipts {
        apply_registry_policy_publications_through(
            current,
            checkpoints,
            checkpoint_index,
            admission_index,
            receipt.previous_version.revision,
            historical_registry,
            history,
        )?;
        apply_service_publication_receipt(current, receipt, historical_registry, history)?;
    }
    Ok(())
}

fn apply_service_publication_receipt(
    current: &FleetCoordinatorRegistryRecord,
    receipt: &FleetServicePublicationReceiptRecord,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    if !FleetServicePublicationAuthority::from_receipt(receipt).is_complete() {
        return Err(receipt_invariant(
            "Fleet-service publication receipt authority is incomplete",
        ));
    }
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
    )?;
    if receipt.previous_version != previous_version {
        return Err(receipt_invariant(
            "Fleet-service publication source differs from canonical history",
        ));
    }
    let next_registry = if receipt.services == historical_registry.services {
        historical_registry.clone()
    } else if historical_registry.services.is_empty() {
        FleetRegistryOps::compile_initial_services(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            historical_registry,
            receipt.services.clone(),
        )
        .map_err(|_| {
            receipt_invariant("initial Fleet-service publication target cannot be rederived")
        })?
    } else {
        FleetRegistryOps::compile_service_additions(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            historical_registry,
            receipt.services.clone(),
        )
        .map_err(|_| {
            receipt_invariant("scale-out Fleet-service publication target cannot be rederived")
        })?
    };
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    if receipt.version != version {
        return Err(receipt_invariant(
            "Fleet-service publication response differs from canonical history",
        ));
    }
    *historical_registry = next_registry.clone();
    if history
        .last()
        .is_none_or(|point| point.registry != next_registry)
    {
        history.push(FleetRegistryHistoryPoint {
            registry: next_registry,
            version,
        });
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct FleetServicePublicationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: canic_core::ids::ComponentDeploymentConfigurationDigest,
    root_receipt_content_hashes: &'a [[u8; 32]],
    services: &'a [canic_core::dto::fleet_registry::FleetServiceBinding],
}

impl<'a> FleetServicePublicationAuthority<'a> {
    fn from_receipt(receipt: &'a FleetServicePublicationReceiptRecord) -> Self {
        Self {
            operation_id: receipt.operation_id,
            plan_hash: receipt.plan_hash,
            configuration_digest: receipt.configuration_digest,
            root_receipt_content_hashes: &receipt.root_receipt_content_hashes,
            services: &receipt.services,
        }
    }

    fn is_complete(&self) -> bool {
        let identity_is_complete = [
            self.operation_id != [0; 32],
            self.plan_hash != [0; 32],
            self.configuration_digest.as_bytes() != &[0; 32],
        ]
        .into_iter()
        .all(|fact| fact);
        let receipt_hashes_are_complete = [
            !self.root_receipt_content_hashes.is_empty(),
            self.root_receipt_content_hashes.len() <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
            self.root_receipt_content_hashes
                .iter()
                .all(|hash| hash != &[0; 32]),
        ]
        .into_iter()
        .all(|fact| fact);
        identity_is_complete && receipt_hashes_are_complete
    }
}

fn apply_lifecycle_receipt(
    current: &FleetCoordinatorRegistryRecord,
    lifecycle: FleetSubnetRootLifecycleReceipt<'_>,
    historical_registry: &mut FleetRegistry,
    history: &mut Vec<FleetRegistryHistoryPoint>,
) -> Result<(), InternalError> {
    let previous_version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
    )
    .map_err(|_| receipt_invariant("root lifecycle source version cannot be derived"))?;
    let (next_registry, expected_response) = match lifecycle {
        FleetSubnetRootLifecycleReceipt::Draining(receipt) => {
            apply_draining_receipt(current, historical_registry, previous_version, receipt)?
        }
        FleetSubnetRootLifecycleReceipt::Removed(receipt) => apply_removal_receipt(
            current,
            historical_registry,
            history,
            previous_version,
            receipt,
        )?,
    };
    if !expected_response.matches(lifecycle) {
        return Err(receipt_invariant(
            "root lifecycle publication response differs from canonical history",
        ));
    }
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )
    .map_err(|_| receipt_invariant("root lifecycle target version cannot be derived"))?;
    *historical_registry = next_registry.clone();
    history.push(FleetRegistryHistoryPoint {
        registry: next_registry,
        version,
    });
    Ok(())
}

fn apply_draining_receipt(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &FleetRegistry,
    previous_version: FleetRegistryVersion,
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
) -> Result<(FleetRegistry, FleetSubnetRootLifecycleResponse), InternalError> {
    let reservation = draining_reservation_for_publication(current, &receipt.request)
        .map_err(|_| receipt_invariant("root draining publication reservation is missing"))?;
    validate_draining_publication_request(
        historical_registry,
        &previous_version,
        &receipt.request,
        reservation,
    )
    .map_err(|_| {
        receipt_invariant("root draining publication request differs from canonical history")
    })?;
    let next_registry = FleetRegistryOps::compile_draining(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
        receipt.request.root_draining.fleet_subnet_root,
    )
    .map_err(|_| receipt_invariant("root draining target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    let response =
        FleetSubnetRootLifecycleResponse::Draining(FleetSubnetRootDrainingPublicationResponse {
            root_draining: receipt.request.root_draining.clone(),
            previous_version,
            version,
        });
    Ok((next_registry, response))
}

fn apply_removal_receipt(
    current: &FleetCoordinatorRegistryRecord,
    historical_registry: &FleetRegistry,
    history: &[FleetRegistryHistoryPoint],
    previous_version: FleetRegistryVersion,
    receipt: &FleetSubnetRootRemovalPublicationReceiptRecord,
) -> Result<(FleetRegistry, FleetSubnetRootLifecycleResponse), InternalError> {
    validate_removal_publication_request(
        historical_registry,
        &previous_version,
        &current.root_draining_publication_receipts,
        history,
        &receipt.request,
    )
    .map_err(|_| {
        receipt_invariant("root removal publication request differs from canonical history")
    })?;
    let next_registry = FleetRegistryOps::compile_removed(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        historical_registry,
        receipt.request.final_inventory.fleet_subnet_root,
    )
    .map_err(|_| receipt_invariant("root removal target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &next_registry,
    )?;
    let response =
        FleetSubnetRootLifecycleResponse::Removed(FleetSubnetRootRemovalPublicationResponse {
            final_inventory: receipt.request.final_inventory.clone(),
            previous_version,
            version,
        });
    Ok((next_registry, response))
}

enum FleetSubnetRootLifecycleResponse {
    Draining(FleetSubnetRootDrainingPublicationResponse),
    Removed(FleetSubnetRootRemovalPublicationResponse),
}

impl FleetSubnetRootLifecycleResponse {
    fn matches(&self, receipt: FleetSubnetRootLifecycleReceipt<'_>) -> bool {
        match (self, receipt) {
            (Self::Draining(expected), FleetSubnetRootLifecycleReceipt::Draining(receipt)) => {
                expected == &receipt.response
            }
            (Self::Removed(expected), FleetSubnetRootLifecycleReceipt::Removed(receipt)) => {
                expected == &receipt.response
            }
            _ => false,
        }
    }
}

fn canonical_lifecycle_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<Vec<FleetSubnetRootLifecycleReceipt<'_>>, InternalError> {
    validate_lifecycle_receipt_identities(current)?;
    let mut receipts = current
        .root_draining_publication_receipts
        .iter()
        .map(FleetSubnetRootLifecycleReceipt::Draining)
        .chain(
            current
                .root_removal_publication_receipts
                .iter()
                .map(FleetSubnetRootLifecycleReceipt::Removed),
        )
        .collect::<Vec<_>>();
    receipts.sort_by_key(|receipt| receipt.revision());
    if receipts
        .windows(2)
        .any(|pair| pair[0].revision() >= pair[1].revision())
    {
        return Err(receipt_invariant(
            "root lifecycle publication revisions are not unique and increasing",
        ));
    }
    Ok(receipts)
}

fn validate_lifecycle_receipt_identities(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let mut draining_identities = Vec::new();
    for receipt in &current.root_draining_publication_receipts {
        let identity = FleetSubnetRootDrainingIdentity::from_publication_request(&receipt.request);
        if draining_identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "root draining publication identity is not unique",
            ));
        }
        draining_identities.push(identity);
    }
    let mut removal_identities = Vec::new();
    for receipt in &current.root_removal_publication_receipts {
        let identity = FleetSubnetRootRemovalPublicationIdentity::from_request(&receipt.request);
        if removal_identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "root removal publication identity is not unique",
            ));
        }
        removal_identities.push(identity);
    }
    Ok(())
}

pub(super) fn validate_root_join_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.root_join_receipts.len() != current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Registry root rows and durable join receipts differ",
        ));
    }

    let historical_registry = historical_joining_registry(current)?;
    for receipt in &current.root_join_receipts {
        let matching = current
            .registry
            .fleet_subnet_roots
            .iter()
            .filter(|entry| {
                entry.placement_subnet == receipt.entry.placement_subnet
                    || entry.fleet_subnet_root == receipt.entry.fleet_subnet_root
            })
            .collect::<Vec<_>>();
        if matching.len() != 1 || !same_root_authority(matching[0], &receipt.entry) {
            return Err(receipt_invariant(
                "Fleet Registry join receipt differs from the current root authority",
            ));
        }
    }
    if historical_registry.fleet_subnet_roots.len() != current.registry.fleet_subnet_roots.len() {
        return Err(receipt_invariant(
            "Fleet Registry join receipt history is incomplete",
        ));
    }
    Ok(())
}

fn historical_joining_registry(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<FleetRegistry, InternalError> {
    let mut historical_registry = FleetRegistryOps::compile_genesis(
        &current.configured_app,
        current.authority.clone(),
        &current
            .component_deployment_configuration
            .component_topology,
        current.initial_admission_policy.clone(),
    )
    .map_err(|_| receipt_invariant("Fleet Registry join receipt genesis is not canonical"))?;
    for receipt in &current.root_join_receipts {
        if receipt.entry.status != FleetSubnetRootStatus::Joining {
            return Err(receipt_invariant(
                "Fleet Registry join receipt does not retain its original Joining row",
            ));
        }
        historical_registry = FleetRegistryOps::compile_joining(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &historical_registry,
            receipt.entry.clone(),
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt history is not canonical"))?;
        let historical_version = FleetRegistryOps::version(
            &current.authority,
            &current
                .component_deployment_configuration
                .component_topology,
            &historical_registry,
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt version cannot be derived"))?;
        if receipt.version != historical_version {
            return Err(receipt_invariant(
                "Fleet Registry join receipt version differs from its historical snapshot",
            ));
        }
    }
    Ok(historical_registry)
}

#[derive(Eq, PartialEq)]
struct FleetSubnetRootImmutableAuthority<'a> {
    placement_subnet: SubnetId,
    fleet_subnet_root: Principal,
    component_admissions: &'a [canic_core::ids::ComponentSpecAdmission],
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
    limits: &'a canic_core::ids::FleetSubnetRootLimits,
}

impl<'a> From<&'a FleetSubnetRootEntry> for FleetSubnetRootImmutableAuthority<'a> {
    fn from(entry: &'a FleetSubnetRootEntry) -> Self {
        Self {
            placement_subnet: entry.placement_subnet,
            fleet_subnet_root: entry.fleet_subnet_root,
            component_admissions: &entry.component_admissions,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
            limits: &entry.limits,
        }
    }
}

fn same_root_authority(left: &FleetSubnetRootEntry, right: &FleetSubnetRootEntry) -> bool {
    FleetSubnetRootImmutableAuthority::from(left) == FleetSubnetRootImmutableAuthority::from(right)
}
