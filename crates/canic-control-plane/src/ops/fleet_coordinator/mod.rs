//! Module: ops::fleet_coordinator
//!
//! Responsibility: validate, compile, commit, and read Fleet Coordinator Registry state.
//! Does not own: endpoint authorization, multi-step lifecycle orchestration, or root effects.
//! Boundary: workflow supplies protected init facts and receives canonical Registry projections.

use crate::{
    dto::fleet_coordinator::FleetCoordinatorInitArgs,
    storage::stable::fleet_coordinator::{
        FleetCoordinatorCommitError, FleetCoordinatorCommitOutcome, FleetCoordinatorRegistryRecord,
        FleetCoordinatorRegistryStore, FleetRegistryActivationReceiptRecord,
        FleetSubnetRootDrainingPublicationReceiptRecord, FleetSubnetRootJoinReceiptRecord,
    },
};
use candid::Principal;
use canic_core::{
    control_plane_support::{
        error::{InternalError, InternalErrorOrigin},
        ops::fleet_registry::FleetRegistryOps,
    },
    dto::fleet_registry::{
        FleetRegistry, FleetRegistryActivationRequest, FleetRegistryActivationResponse,
        FleetRegistryManifest, FleetRegistrySnapshotResponse, FleetRegistryVersion,
        FleetSubnetRootDrainingPublicationRequest, FleetSubnetRootDrainingPublicationResponse,
        FleetSubnetRootEntry, FleetSubnetRootJoinRequest, FleetSubnetRootJoinResponse,
        FleetSubnetRootSnapshotAcknowledgement, FleetSubnetRootSnapshotAcknowledgementRequest,
        FleetSubnetRootStatus,
    },
    ids::{ComponentTopologyDigest, FleetSubnetRootReleaseSet, SubnetId},
};

///
/// FleetCoordinatorOps
///
/// Single-step Coordinator state and canonical Registry operations.
///

pub struct FleetCoordinatorOps;

impl FleetCoordinatorOps {
    pub(crate) fn compile_genesis(
        args: FleetCoordinatorInitArgs,
        coordinator_canister: Principal,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if args.authority.binding.coordinator != coordinator_canister {
            return Err(InternalError::invalid_input(
                "Fleet Coordinator authority principal does not match the installed canister",
            ));
        }
        args.component_topology
            .canonical_bytes()
            .map_err(|error| InternalError::invalid_input(error.to_string()))?;
        let registry = FleetRegistryOps::compile_genesis(
            &args.configured_app,
            args.authority.clone(),
            &args.component_topology,
        )?;
        Ok(FleetCoordinatorRegistryRecord {
            configured_app: args.configured_app,
            authority: args.authority,
            component_topology: args.component_topology,
            registry,
            root_join_receipts: Vec::new(),
            root_snapshot_acknowledgements: Vec::new(),
            registry_activation_receipt: None,
            root_draining_publication_receipts: Vec::new(),
        })
    }

    pub(crate) fn commit_genesis(
        record: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_genesis(record).map_err(|_| {
            InternalError::conflict(
                "Fleet Coordinator already contains different protected Registry state",
            )
        })
    }

    pub(crate) fn registry() -> Result<FleetRegistry, InternalError> {
        Ok(Self::current()?.registry)
    }

    pub(crate) fn join_root(
        request: FleetSubnetRootJoinRequest,
    ) -> Result<FleetSubnetRootJoinResponse, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = current.root_join_receipts.iter().find(|receipt| {
            receipt.entry.placement_subnet == request.entry.placement_subnet
                || receipt.entry.fleet_subnet_root == request.entry.fleet_subnet_root
        }) {
            if receipt.entry == request.entry {
                return Ok(FleetSubnetRootJoinResponse {
                    entry: receipt.entry.clone(),
                    version: receipt.version.clone(),
                });
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root join identity already has different protected authority",
            ));
        }
        if current.registry_activation_receipt.is_some() {
            return Err(InternalError::conflict(
                "initial Fleet Registry activation already committed",
            ));
        }

        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root join expected Registry version is stale",
            ));
        }
        let next_registry = FleetRegistryOps::compile_joining(
            &current.authority,
            &current.component_topology,
            &current.registry,
            request.entry.clone(),
        )?;
        if next_registry == current.registry {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "Fleet Registry contains a root without its durable join receipt",
            ));
        }
        let version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &next_registry,
        )?;
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_join_receipts
            .push(FleetSubnetRootJoinReceiptRecord {
                entry: request.entry.clone(),
                version: version.clone(),
            });
        next.root_snapshot_acknowledgements.clear();
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(FleetSubnetRootJoinResponse {
            entry: request.entry,
            version,
        })
    }

    pub(crate) fn manifest() -> Result<FleetRegistryManifest, InternalError> {
        let current = Self::current()?;
        FleetRegistryOps::manifest(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )
    }

    pub(crate) fn snapshot_for_root(
        caller: Principal,
    ) -> Result<FleetRegistrySnapshotResponse, InternalError> {
        let current = Self::current()?;
        require_snapshot_root(&current, caller)?;
        let manifest = FleetRegistryOps::manifest(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        let version = FleetRegistryVersion {
            authority: manifest.authority.clone(),
            revision: manifest.revision,
            content_hash: manifest.content_hash,
        };
        Ok(FleetRegistrySnapshotResponse {
            registry: current.registry,
            manifest,
            version,
        })
    }

    pub(crate) fn acknowledge_root_snapshot(
        caller: Principal,
        request: FleetSubnetRootSnapshotAcknowledgementRequest,
    ) -> Result<FleetSubnetRootSnapshotAcknowledgement, InternalError> {
        let current = Self::current()?;
        require_all_roots_joining(&current)?;
        require_joining_root(&current, caller)?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        if request.version != current_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root snapshot acknowledgement is stale",
            ));
        }
        let acknowledgement = FleetSubnetRootSnapshotAcknowledgement {
            fleet_subnet_root: caller,
            version: current_version,
        };
        if let Some(existing) = current
            .root_snapshot_acknowledgements
            .iter()
            .find(|existing| existing.fleet_subnet_root == caller)
        {
            if existing == &acknowledgement {
                return Ok(existing.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root already acknowledged different Registry authority",
            ));
        }

        let mut next = current.clone();
        next.root_snapshot_acknowledgements
            .push(acknowledgement.clone());
        next.root_snapshot_acknowledgements.sort_by(|left, right| {
            left.fleet_subnet_root
                .as_slice()
                .cmp(right.fleet_subnet_root.as_slice())
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(acknowledgement)
    }

    pub(crate) fn activate_registry(
        request: FleetRegistryActivationRequest,
    ) -> Result<FleetRegistryActivationResponse, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = &current.registry_activation_receipt {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Registry activation already committed against different authority",
            ));
        }
        require_all_roots_joining(&current)?;
        let current_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        if request.expected_registry != current_version {
            return Err(InternalError::conflict(
                "Fleet Registry activation expected version is stale",
            ));
        }
        require_complete_snapshot_acknowledgements(&current, &current_version)?;

        let next_registry = FleetRegistryOps::compile_active(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &next_registry,
        )?;
        let response = FleetRegistryActivationResponse {
            previous_version: current_version,
            version,
        };
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_snapshot_acknowledgements.clear();
        next.registry_activation_receipt = Some(FleetRegistryActivationReceiptRecord {
            request,
            response: response.clone(),
        });
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn publish_root_draining(
        request: FleetSubnetRootDrainingPublicationRequest,
    ) -> Result<FleetSubnetRootDrainingPublicationResponse, InternalError> {
        let current = Self::current()?;
        if let Some(receipt) = current
            .root_draining_publication_receipts
            .iter()
            .find(|receipt| draining_publication_identity_matches(receipt, &request))
        {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication identity already has different authority",
            ));
        }
        if current.registry_activation_receipt.is_none() {
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication requires an active Fleet Registry",
            ));
        }
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        if request.expected_registry != previous_version {
            return Err(InternalError::conflict(
                "Fleet Subnet Root draining publication expected Registry version is stale",
            ));
        }
        validate_draining_publication_request(&current.registry, &previous_version, &request)
            .map_err(InternalError::invalid_input)?;

        let next_registry = FleetRegistryOps::compile_draining(
            &current.authority,
            &current.component_topology,
            &current.registry,
            request.root_draining.fleet_subnet_root,
        )?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &next_registry,
        )?;
        let response = FleetSubnetRootDrainingPublicationResponse {
            root_draining: request.root_draining.clone(),
            previous_version,
            version,
        };
        let mut next = current.clone();
        next.registry = next_registry;
        next.root_draining_publication_receipts.push(
            FleetSubnetRootDrainingPublicationReceiptRecord {
                request,
                response: response.clone(),
            },
        );
        let next = Self::validate_current(next)?;
        Self::commit_transition(&current, next)?;
        Ok(response)
    }

    pub(crate) fn root_snapshot_acknowledgements()
    -> Result<Vec<FleetSubnetRootSnapshotAcknowledgement>, InternalError> {
        Ok(Self::current()?.root_snapshot_acknowledgements)
    }

    pub(crate) fn version() -> Result<FleetRegistryVersion, InternalError> {
        let current = Self::current()?;
        FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )
    }

    fn current() -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        FleetCoordinatorRegistryStore::export()
            .current
            .ok_or_else(|| {
                InternalError::unavailable("Fleet Coordinator genesis is not initialized")
            })
            .and_then(Self::validate_current)
    }

    fn validate_current(
        current: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorRegistryRecord, InternalError> {
        if current.authority != current.registry.authority {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Fleet Coordinator authority does not match its Fleet Registry",
            ));
        }
        if current.configured_app != current.authority.binding.fleet.app {
            return Err(InternalError::invariant(
                InternalErrorOrigin::Storage,
                "stored Fleet Coordinator App does not match its authority",
            ));
        }
        FleetRegistryOps::validate(
            &current.authority,
            &current.component_topology,
            &current.registry,
        )?;
        validate_root_join_receipts(&current)?;
        validate_root_snapshot_acknowledgements(&current)?;
        validate_registry_lifecycle_history(&current)?;
        Ok(current)
    }

    fn commit_transition(
        current: &FleetCoordinatorRegistryRecord,
        next: FleetCoordinatorRegistryRecord,
    ) -> Result<FleetCoordinatorCommitOutcome, InternalError> {
        FleetCoordinatorRegistryStore::commit_transition(current, next).map_err(|error| match error
        {
            FleetCoordinatorCommitError::ConflictingState => InternalError::conflict(
                "Fleet Coordinator Registry changed before the requested transition committed",
            ),
            FleetCoordinatorCommitError::Uninitialized => {
                InternalError::unavailable("Fleet Coordinator genesis is not initialized")
            }
        })
    }
}

fn require_snapshot_root(
    current: &FleetCoordinatorRegistryRecord,
    caller: Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| {
            entry.fleet_subnet_root == caller && entry.status != FleetSubnetRootStatus::Removed
        })
        .ok_or_else(|| {
            InternalError::forbidden(
                "caller is not a current Fleet Subnet Root in the Fleet Registry",
            )
        })
}

fn require_joining_root(
    current: &FleetCoordinatorRegistryRecord,
    caller: Principal,
) -> Result<&FleetSubnetRootEntry, InternalError> {
    current
        .registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| {
            entry.fleet_subnet_root == caller && entry.status == FleetSubnetRootStatus::Joining
        })
        .ok_or_else(|| {
            InternalError::forbidden(
                "caller is not a Joining Fleet Subnet Root in the current Registry",
            )
        })
}

fn require_all_roots_joining(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.registry.fleet_subnet_roots.is_empty()
        || current
            .registry
            .fleet_subnet_roots
            .iter()
            .any(|entry| entry.status != FleetSubnetRootStatus::Joining)
    {
        return Err(InternalError::conflict(
            "Fleet Registry snapshot synchronization requires a non-empty all-Joining root set",
        ));
    }
    Ok(())
}

fn validate_root_snapshot_acknowledgements(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let version = FleetRegistryOps::version(
        &current.authority,
        &current.component_topology,
        &current.registry,
    )?;
    let mut previous: Option<Principal> = None;
    for acknowledgement in &current.root_snapshot_acknowledgements {
        if acknowledgement.version != version
            || previous
                .as_ref()
                .is_some_and(|root| root.as_slice() >= acknowledgement.fleet_subnet_root.as_slice())
            || require_joining_root(current, acknowledgement.fleet_subnet_root).is_err()
        {
            return Err(receipt_invariant(
                "Fleet Subnet Root snapshot acknowledgements are not canonical",
            ));
        }
        previous = Some(acknowledgement.fleet_subnet_root);
    }
    Ok(())
}

fn require_complete_snapshot_acknowledgements(
    current: &FleetCoordinatorRegistryRecord,
    version: &FleetRegistryVersion,
) -> Result<(), InternalError> {
    if current.root_snapshot_acknowledgements.len() != current.registry.fleet_subnet_roots.len()
        || current.registry.fleet_subnet_roots.iter().any(|entry| {
            !current
                .root_snapshot_acknowledgements
                .iter()
                .any(|acknowledgement| {
                    acknowledgement.fleet_subnet_root == entry.fleet_subnet_root
                        && &acknowledgement.version == version
                })
        })
    {
        return Err(InternalError::conflict(
            "Fleet Registry activation requires every current root acknowledgement",
        ));
    }
    Ok(())
}

fn validate_registry_lifecycle_history(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let joining = historical_joining_registry(current)?;
    let Some(receipt) = &current.registry_activation_receipt else {
        if !current.root_draining_publication_receipts.is_empty() || current.registry != joining {
            return Err(receipt_invariant(
                "Fleet Registry contains transitioned roots without an activation receipt",
            ));
        }
        return Ok(());
    };
    if !current.root_snapshot_acknowledgements.is_empty() {
        return Err(receipt_invariant(
            "active Fleet Registry retains stale Joining acknowledgements",
        ));
    }
    let previous_version =
        FleetRegistryOps::version(&current.authority, &current.component_topology, &joining)
            .map_err(|_| receipt_invariant("activation source version cannot be derived"))?;
    let mut historical_registry =
        FleetRegistryOps::compile_active(&current.authority, &current.component_topology, &joining)
            .map_err(|_| receipt_invariant("activation target Registry cannot be derived"))?;
    let version = FleetRegistryOps::version(
        &current.authority,
        &current.component_topology,
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
    let mut draining_identities =
        Vec::with_capacity(current.root_draining_publication_receipts.len());
    for receipt in &current.root_draining_publication_receipts {
        let identity = FleetSubnetRootDrainingPublicationIdentity::from_request(&receipt.request);
        if draining_identities
            .iter()
            .any(|existing| identity.conflicts_with(*existing))
        {
            return Err(receipt_invariant(
                "root draining publication identity is not unique",
            ));
        }
        draining_identities.push(identity);
        let previous_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &historical_registry,
        )
        .map_err(|_| receipt_invariant("root draining source version cannot be derived"))?;
        validate_draining_publication_request(
            &historical_registry,
            &previous_version,
            &receipt.request,
        )
        .map_err(|_| {
            receipt_invariant("root draining publication request differs from canonical history")
        })?;
        historical_registry = FleetRegistryOps::compile_draining(
            &current.authority,
            &current.component_topology,
            &historical_registry,
            receipt.request.root_draining.fleet_subnet_root,
        )
        .map_err(|_| receipt_invariant("root draining target Registry cannot be derived"))?;
        let version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
            &historical_registry,
        )
        .map_err(|_| receipt_invariant("root draining target version cannot be derived"))?;
        let expected_response = FleetSubnetRootDrainingPublicationResponse {
            root_draining: receipt.request.root_draining.clone(),
            previous_version,
            version,
        };
        if receipt.response != expected_response {
            return Err(receipt_invariant(
                "root draining publication response differs from canonical history",
            ));
        }
    }
    if current.registry != historical_registry {
        return Err(receipt_invariant(
            "Fleet Registry head differs from its canonical lifecycle history",
        ));
    }
    Ok(())
}

fn validate_root_join_receipts(
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
        &current.component_topology,
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
            &current.component_topology,
            &historical_registry,
            receipt.entry.clone(),
        )
        .map_err(|_| receipt_invariant("Fleet Registry join receipt history is not canonical"))?;
        let historical_version = FleetRegistryOps::version(
            &current.authority,
            &current.component_topology,
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

#[derive(Eq, PartialEq)]
struct FleetSubnetRootDrainingAuthority<'a> {
    fleet_subnet_root: Principal,
    placement_subnet: SubnetId,
    active_registry: &'a FleetRegistryVersion,
    component_topology_digest: ComponentTopologyDigest,
    active_release_set: FleetSubnetRootReleaseSet,
}

impl<'a> FleetSubnetRootDrainingAuthority<'a> {
    const fn from_registry(
        entry: &'a FleetSubnetRootEntry,
        version: &'a FleetRegistryVersion,
    ) -> Self {
        Self {
            fleet_subnet_root: entry.fleet_subnet_root,
            placement_subnet: entry.placement_subnet,
            active_registry: version,
            component_topology_digest: entry.component_topology_digest,
            active_release_set: entry.active_release_set,
        }
    }

    const fn from_publication(request: &'a FleetSubnetRootDrainingPublicationRequest) -> Self {
        let draining = &request.root_draining;
        Self {
            fleet_subnet_root: draining.fleet_subnet_root,
            placement_subnet: draining.placement_subnet,
            active_registry: &draining.active_registry,
            component_topology_digest: draining.component_topology_digest,
            active_release_set: draining.active_release_set,
        }
    }
}

fn draining_publication_identity_matches(
    receipt: &FleetSubnetRootDrainingPublicationReceiptRecord,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> bool {
    FleetSubnetRootDrainingPublicationIdentity::from_request(&receipt.request).conflicts_with(
        FleetSubnetRootDrainingPublicationIdentity::from_request(request),
    )
}

#[derive(Clone, Copy)]
struct FleetSubnetRootDrainingPublicationIdentity {
    fleet_subnet_root: Principal,
    operation_id: [u8; 32],
}

impl FleetSubnetRootDrainingPublicationIdentity {
    const fn from_request(request: &FleetSubnetRootDrainingPublicationRequest) -> Self {
        Self {
            fleet_subnet_root: request.root_draining.fleet_subnet_root,
            operation_id: request.root_draining.operation_id,
        }
    }

    fn conflicts_with(self, other: Self) -> bool {
        self.fleet_subnet_root == other.fleet_subnet_root || self.operation_id == other.operation_id
    }
}

fn validate_draining_publication_request(
    registry: &FleetRegistry,
    version: &FleetRegistryVersion,
    request: &FleetSubnetRootDrainingPublicationRequest,
) -> Result<(), &'static str> {
    let draining = &request.root_draining;
    if request.expected_registry != *version || draining.active_registry != *version {
        return Err("Fleet Subnet Root draining publication names stale Registry authority");
    }
    let target = registry
        .fleet_subnet_roots
        .iter()
        .find(|entry| entry.fleet_subnet_root == draining.fleet_subnet_root)
        .ok_or("Fleet Subnet Root draining publication target is missing")?;
    if target.status != FleetSubnetRootStatus::Active {
        return Err("Fleet Subnet Root draining publication target is not Active");
    }
    let expected_authority = FleetSubnetRootDrainingAuthority::from_registry(target, version);
    if FleetSubnetRootDrainingAuthority::from_publication(request) != expected_authority {
        return Err("Fleet Subnet Root draining receipt differs from Registry root authority");
    }
    if draining.operation_id == [0; 32]
        || draining.started_at_ns == 0
        || draining.next_allocation_sequence == 0
    {
        return Err("Fleet Subnet Root draining receipt contains non-positive operation facts");
    }
    let component_instances = draining
        .reserved_component_instances
        .checked_add(draining.committed_component_instances)
        .ok_or("Fleet Subnet Root draining Component Instance count overflowed")?;
    if component_instances > target.limits.maximum_component_instances {
        return Err("Fleet Subnet Root draining Component Instance count exceeds its limit");
    }
    if draining.next_allocation_sequence <= u64::from(component_instances) {
        return Err("Fleet Subnet Root draining allocation sequence precedes its live instances");
    }
    let allocated_canisters = component_instances
        .checked_add(draining.managed_descendants)
        .ok_or("Fleet Subnet Root draining managed canister count overflowed")?;
    if draining.known_created_component_canisters > allocated_canisters {
        return Err("Fleet Subnet Root draining created canisters exceed allocated canisters");
    }
    let managed_canisters = allocated_canisters
        .checked_add(1)
        .ok_or("Fleet Subnet Root draining managed canister count overflowed")?;
    if managed_canisters > target.limits.maximum_managed_canisters {
        return Err("Fleet Subnet Root draining managed canisters exceed the root limit");
    }
    if draining.root_registry_encoded_bytes > target.limits.maximum_registry_bytes {
        return Err("Fleet Subnet Root draining Registry bytes exceed the root limit");
    }
    Ok(())
}

fn receipt_invariant(message: &'static str) -> InternalError {
    InternalError::invariant(InternalErrorOrigin::Storage, message)
}
