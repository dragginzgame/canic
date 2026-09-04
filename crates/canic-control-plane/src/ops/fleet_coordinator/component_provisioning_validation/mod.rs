//! Module: ops::fleet_coordinator::component_provisioning_validation
//!
//! Responsibility: validate retained Component provisioning, scale-out, Directory, and activation evidence.
//! Does not own: Coordinator storage, commits, orchestration, transport, or downstream effects.
//! Boundary: derives canonical authority from the current record and rejects non-monotonic retained evidence.

use super::*;

fn validate_component_provisioning_root_acceptance_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = component_provisioning_root_acceptance_progress(record)?;
    if progress.planned_at_ns == 0 {
        return Err(receipt_invariant(
            "Fleet Component provisioning planned time is zero",
        ));
    }
    if progress.accepted_root_count > progress.root_batch_count {
        return Err(receipt_invariant(
            "Fleet Component accepted root count exceeds its complete plan",
        ));
    }
    let mut previous_recorded_at_ns = progress.planned_at_ns;
    for (index, acceptance) in progress.acceptances.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("accepted root index does not fit u32"))?;
        let batch = root_batch(record, root_index)?;
        validate_root_acceptance_response(record, batch, &acceptance.response).map_err(|_| {
            receipt_invariant("stored root acceptance differs from its exact plan batch")
        })?;
        if acceptance.started_at_ns < previous_recorded_at_ns {
            return Err(receipt_invariant(
                "Fleet Component root acceptance time evidence is invalid",
            ));
        }
        validate_root_acceptance_observation(
            acceptance.started_at_ns,
            &acceptance.response,
            acceptance.recorded_at_ns,
        )
        .map_err(|_| {
            receipt_invariant("Fleet Component root acceptance time evidence is invalid")
        })?;
        previous_recorded_at_ns = acceptance.recorded_at_ns;
    }
    validate_root_acceptance_phase(record, &progress, previous_recorded_at_ns)
}

fn validate_root_acceptance_phase(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootAcceptanceProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    match progress.phase {
        FleetComponentProvisioningPhase::Planned => Ok(()),
        FleetComponentProvisioningPhase::AcceptingRoots => {
            if matches!(
                (progress.accepted_root_count, progress.in_flight),
                (0, None)
            ) {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance has neither progress nor pre-call intent",
                ));
            }
            if progress.accepted_root_count >= progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance remained nonterminal after every root",
                ));
            }
            let Some(intent) = progress.in_flight else {
                return Ok(());
            };
            if intent.root_index != progress.accepted_root_count {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent differs from its durable cursor",
                ));
            }
            let batch = root_batch(record, intent.root_index)?;
            if intent.fleet_subnet_root != batch.root.fleet_subnet_root {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent names a different root",
                ));
            }
            if intent.started_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component root acceptance intent time regressed",
                ));
            }
            Ok(())
        }
        FleetComponentProvisioningPhase::RootsAccepted => {
            if progress.accepted_root_count != progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted state lacks complete root evidence",
                ));
            }
            let completed_at_ns = progress
                .roots_accepted_at_ns
                .ok_or_else(|| receipt_invariant("Fleet Component RootsAccepted time is absent"))?;
            if completed_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted time precedes root evidence",
                ));
            }
            Ok(())
        }
        FleetComponentProvisioningPhase::ProvisioningRoots
        | FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            if progress.accepted_root_count != progress.root_batch_count {
                return Err(receipt_invariant(
                    "Fleet Component post-acceptance state lacks complete root evidence",
                ));
            }
            let completed_at_ns = progress
                .roots_accepted_at_ns
                .ok_or_else(|| receipt_invariant("Fleet Component RootsAccepted time is absent"))?;
            if completed_at_ns < previous_recorded_at_ns {
                return Err(receipt_invariant(
                    "Fleet Component RootsAccepted time precedes root evidence",
                ));
            }
            Ok(())
        }
    }
}

fn validate_component_provisioning_root_provision_state(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let acceptance = component_provisioning_root_acceptance_progress(record)?;
    let progress = component_provisioning_root_provision_progress(record)?;
    match acceptance.phase {
        FleetComponentProvisioningPhase::Planned
        | FleetComponentProvisioningPhase::AcceptingRoots => {
            if progress.provisioned_root_count != 0
                || progress.current_response.is_some()
                || progress.in_flight.is_some()
                || progress.roots_accepted_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "root provisioning evidence exists before complete root acceptance",
                ));
            }
            return Ok(());
        }
        FleetComponentProvisioningPhase::RootsAccepted => {
            if progress.provisioned_root_count != 0
                || progress.current.is_some()
                || progress.in_flight.is_some()
                || progress.current_response.is_none()
            {
                return Err(receipt_invariant(
                    "RootsAccepted state contains invalid root provisioning evidence",
                ));
            }
            return Ok(());
        }
        FleetComponentProvisioningPhase::ProvisioningRoots
        | FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {}
    }
    let roots_accepted_at_ns = progress.roots_accepted_at_ns.ok_or_else(|| {
        receipt_invariant("root provisioning state lacks RootsAccepted time authority")
    })?;
    if progress.provisioned_root_count > acceptance.root_batch_count {
        return Err(receipt_invariant(
            "provisioned root count exceeds the complete plan",
        ));
    }
    let previous_observed_at_ns = validate_root_provision_receipts(
        configuration,
        record,
        &progress.provisions,
        roots_accepted_at_ns,
    )?;
    validate_current_root_provision_record(record, &progress, previous_observed_at_ns)?;
    validate_root_provision_intent(record, &progress)?;
    match acceptance.phase {
        FleetComponentProvisioningPhase::ProvisioningRoots => {
            if progress.provisioned_root_count >= acceptance.root_batch_count
                || progress.components_provisioned_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "root provisioning remained nonterminal after every planned root",
                ));
            }
        }
        FleetComponentProvisioningPhase::ComponentsProvisioned
        | FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            validate_terminal_component_provisioning(
                configuration,
                source_registry,
                record,
                &acceptance,
                &progress,
                previous_observed_at_ns,
            )?;
        }
        _ => unreachable!("pre-provisioning phases returned above"),
    }
    Ok(())
}

fn validate_terminal_component_provisioning(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    source_registry: &FleetRegistry,
    record: &FleetComponentProvisioningRecord,
    acceptance: &FleetComponentProvisioningRootAcceptanceProgress,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    previous_observed_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.provisioned_root_count != acceptance.root_batch_count
        || progress.current_response.is_some()
        || progress.in_flight.is_some()
    {
        return Err(receipt_invariant(
            "ComponentsProvisioned state lacks complete terminal root evidence",
        ));
    }
    let completed_at_ns = progress
        .components_provisioned_at_ns
        .ok_or_else(|| receipt_invariant("ComponentsProvisioned time evidence is absent"))?;
    if completed_at_ns < previous_observed_at_ns {
        return Err(receipt_invariant(
            "ComponentsProvisioned time precedes terminal root evidence",
        ));
    }
    let receipts = progress
        .provisions
        .iter()
        .map(|provision| provision.response.clone())
        .collect::<Vec<_>>();
    compile_component_operation_services(configuration, source_registry, record, &receipts)
        .map_err(|_| {
            receipt_invariant(
                "complete root provisioning receipts do not compile canonical services",
            )
        })?;
    validate_service_publication_progress(acceptance.phase, progress, completed_at_ns)
}

fn validate_service_publication_progress(
    phase: FleetComponentProvisioningPhase,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    components_provisioned_at_ns: u64,
) -> Result<(), InternalError> {
    match phase {
        FleetComponentProvisioningPhase::ComponentsProvisioned => {
            if progress.published_fleet_registry.is_some()
                || progress.service_topology_published_at_ns.is_some()
            {
                return Err(receipt_invariant(
                    "ComponentsProvisioned state contains premature publication evidence",
                ));
            }
        }
        FleetComponentProvisioningPhase::ServiceTopologyPublished
        | FleetComponentProvisioningPhase::ConfirmingDirectories
        | FleetComponentProvisioningPhase::DirectoriesConfirmed
        | FleetComponentProvisioningPhase::ActivatingRuntimes
        | FleetComponentProvisioningPhase::RuntimesActivated => {
            let published_at_ns = progress.service_topology_published_at_ns.ok_or_else(|| {
                receipt_invariant("ServiceTopologyPublished time evidence is absent")
            })?;
            if progress.published_fleet_registry.is_none()
                || published_at_ns < components_provisioned_at_ns
            {
                return Err(receipt_invariant(
                    "ServiceTopologyPublished state contains invalid publication evidence",
                ));
            }
        }
        _ => unreachable!("pre-provisioning phases returned above"),
    }
    Ok(())
}

fn validate_root_provision_receipts(
    configuration: &canic_core::control_plane_support::config::ComponentDeploymentConfiguration,
    record: &FleetComponentProvisioningRecord,
    provisions: &[FleetComponentProvisioningRootProvisionRecord],
    roots_accepted_at_ns: u64,
) -> Result<u64, InternalError> {
    let mut previous_observed_at_ns = roots_accepted_at_ns;
    for (index, provision) in provisions.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("provisioned root index does not fit u32"))?;
        let accepted = component_provisioning_root_acceptance(record, root_index)?;
        if provision.started_at_ns < previous_observed_at_ns
            || provision.recorded_at_ns < provision.started_at_ns
            || provision.response.accepted_at_ns != accepted.response.accepted_at_ns
        {
            return Err(receipt_invariant(
                "stored root Provisioned response time evidence is invalid",
            ));
        }
        let provisioned_at_ns = provision.response.provisioned_at_ns.ok_or_else(|| {
            receipt_invariant("stored root Provisioned response has no completion time")
        })?;
        if provision.recorded_at_ns < provisioned_at_ns {
            return Err(receipt_invariant(
                "stored root Provisioned response time evidence is invalid",
            ));
        }
        FleetServiceBindingOps::validate_provisioned_root_receipt_compiled(
            configuration,
            &record.plan,
            record.operation_id,
            record.plan_hash,
            index,
            &provision.response,
        )
        .map_err(|_| {
            receipt_invariant("stored root Provisioned response differs from its plan batch")
        })?;
        if !root_post_provisioning_progress_is_absent(&provision.response) {
            return Err(receipt_invariant(
                "stored root Provisioned response carries post-provisioning progress",
            ));
        }
        previous_observed_at_ns = provision.recorded_at_ns;
    }
    Ok(previous_observed_at_ns)
}

fn validate_current_root_provision_record(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootProvisionProgress,
    previous_observed_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(current) = &progress.current else {
        return Ok(());
    };
    if current.started_at_ns < previous_observed_at_ns
        || current.recorded_at_ns < current.started_at_ns
    {
        return Err(InternalError::invariant());
    }
    let batch = root_batch(record, progress.provisioned_root_count)?;
    let accepted = component_provisioning_root_acceptance(record, progress.provisioned_root_count)?;
    validate_root_provision_current(record, batch, &accepted, &current.response).map_err(|_| {
        receipt_invariant("current root provisioning response differs from its plan batch")
    })
}

fn validate_root_provision_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentProvisioningRootProvisionProgress,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    if intent.root_index != progress.provisioned_root_count
        || intent.started_at_ns < root_provision_previous_observed_at(progress)?
    {
        return Err(receipt_invariant(
            "root provisioning pre-call intent differs from its durable cursor",
        ));
    }
    let response = progress
        .current_response
        .as_ref()
        .ok_or_else(|| receipt_invariant("root provisioning intent has no current root cursor"))?;
    let expected = root_provision_call(record, intent.root_index, response)?;
    if intent.fleet_subnet_root != expected.fleet_subnet_root || intent.request != expected.request
    {
        return Err(receipt_invariant(
            "root provisioning pre-call intent differs from its exact root request",
        ));
    }
    Ok(())
}

pub(super) fn validate_runtime_activation_response(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    previous: FleetComponentActivationRootProgress,
    previous_activation_started_at_ns: Option<u64>,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    validate_runtime_activation_authority(publication, response)?;
    let actual = root_activation_progress(response);
    if !activation_progress_advances(previous, actual) {
        return Err(InternalError::conflict());
    }
    let activation_started_at_ns = response
        .activation_started_at_ns
        .ok_or_else(InternalError::conflict)?;
    if previous_activation_started_at_ns
        .is_some_and(|expected| expected != activation_started_at_ns)
    {
        return Err(InternalError::conflict());
    }
    let published_at_ns = response.published_at_ns.ok_or_else(|| {
        receipt_invariant("runtime activation publication lacks its completion time")
    })?;
    if activation_started_at_ns < published_at_ns || recorded_at_ns < activation_started_at_ns {
        return Err(InternalError::conflict());
    }
    match response.phase {
        RootComponentProvisioningPhase::Published => {
            let progress_is_exact = [
                !response.root_runtime_active,
                response.activated_component_count <= response.component_count,
                response.activation.is_none(),
                response.runtimes_activated_at_ns.is_none(),
                response.receipt_content_hash == publication.receipt_content_hash,
            ]
            .into_iter()
            .all(|matches| matches);
            if !progress_is_exact {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::RuntimesActive => {
            validate_terminal_runtime_activation(
                record,
                root_index,
                publication,
                response,
                activation_started_at_ns,
                recorded_at_ns,
            )?;
        }
        _ => return Err(InternalError::conflict()),
    }
    Ok(())
}

fn validate_runtime_activation_authority(
    publication: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
) -> Result<(), InternalError> {
    let published = publication.phase == RootComponentProvisioningPhase::Published;
    let authority_is_exact = RootRuntimeActivationAuthority::from_response(response)
        == RootRuntimeActivationAuthority::from_response(publication);
    if !published || !authority_is_exact {
        return Err(InternalError::conflict());
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct RootRuntimeActivationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    fleet_registry: &'a FleetRegistryVersion,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fleet_subnet_root: Principal,
    counts: RootRuntimeActivationCounts,
    result: &'a Option<canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    publication:
        &'a Option<canic_core::dto::component_provisioning::RootComponentPublicationEvidence>,
    accepted_at_ns: u64,
    provisioned_at_ns: Option<u64>,
    published_at_ns: Option<u64>,
}

impl<'a> RootRuntimeActivationAuthority<'a> {
    const fn from_response(response: &'a RootComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: response.operation_id,
            plan_hash: response.plan_hash,
            fleet_registry: &response.fleet_registry,
            configuration_digest: response.configuration_digest,
            fleet_subnet_root: response.fleet_subnet_root,
            counts: RootRuntimeActivationCounts::from_response(response),
            result: &response.result,
            publication: &response.publication,
            accepted_at_ns: response.accepted_at_ns,
            provisioned_at_ns: response.provisioned_at_ns,
            published_at_ns: response.published_at_ns,
        }
    }
}

#[derive(Eq, PartialEq)]
struct RootRuntimeActivationCounts {
    placements: u32,
    components: u32,
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
    published: u32,
}

impl RootRuntimeActivationCounts {
    const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            placements: response.placement_count,
            components: response.component_count,
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
            published: response.published_component_count,
        }
    }
}

fn validate_terminal_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
    activation_started_at_ns: u64,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let activation = response.activation.ok_or_else(InternalError::conflict)?;
    let runtimes_activated_at_ns = response
        .runtimes_activated_at_ns
        .ok_or_else(InternalError::conflict)?;
    let progress_is_terminal = response.root_runtime_active
        && response.activated_component_count == response.component_count;
    let identity_is_exact = activation.component_count == response.component_count
        && activation.fleet_activation_operation_id != [0; 32]
        && activation.initial_inventory_hash != [0; 32];
    let timing_is_exact = terminal_root_activation_timing_is_valid(
        &record.plan.operation,
        activation.root_activated_at_ns,
        response.accepted_at_ns,
        activation_started_at_ns,
        runtimes_activated_at_ns,
    );
    if !(progress_is_terminal && identity_is_exact && timing_is_exact)
        || recorded_at_ns < runtimes_activated_at_ns
    {
        return Err(InternalError::conflict());
    }
    let batch = root_batch(record, root_index)?;
    let expected = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            published_receipt_content_hash: publication.receipt_content_hash,
            activation,
            activation_started_at_ns,
            runtimes_activated_at_ns,
        },
    )?;
    if response.receipt_content_hash != expected {
        return Err(InternalError::conflict());
    }
    Ok(())
}

const fn terminal_root_activation_timing_is_valid(
    operation: &FleetComponentProvisioningOperation,
    root_activated_at_ns: u64,
    accepted_at_ns: u64,
    activation_started_at_ns: u64,
    runtimes_activated_at_ns: u64,
) -> bool {
    if runtimes_activated_at_ns < activation_started_at_ns {
        return false;
    }
    match operation {
        FleetComponentProvisioningOperation::FreshInstall => {
            root_activated_at_ns == runtimes_activated_at_ns
        }
        FleetComponentProvisioningOperation::ScaleOut { .. } => {
            root_activated_at_ns > 0 && root_activated_at_ns <= accepted_at_ns
        }
    }
}

pub(super) fn expected_fleet_directory_content_hash(
    current: &FleetCoordinatorRegistryRecord,
    published_registry: &FleetRegistryVersion,
    root: Principal,
) -> Result<[u8; 32], InternalError> {
    let registry = registry_snapshot_at_version(current, published_registry)?;
    let directory = FleetRegistryOps::directory_for_root(
        &registry.authority,
        &current
            .component_deployment_configuration
            .component_topology,
        &registry,
        root,
    )?;
    RootComponentProvisioningReceiptOps::fleet_directory_content_hash(&directory)
}

pub(super) struct RootDirectoryConfirmationValidationContext<'a> {
    operation: &'a FleetComponentProvisioningRecord,
    published_registry: &'a FleetRegistryVersion,
    root: Principal,
    fleet_directory_content_hash: [u8; 32],
}

impl<'a> RootDirectoryConfirmationValidationContext<'a> {
    pub(super) const fn new(
        operation: &'a FleetComponentProvisioningRecord,
        published_registry: &'a FleetRegistryVersion,
        root: Principal,
        fleet_directory_content_hash: [u8; 32],
    ) -> Self {
        Self {
            operation,
            published_registry,
            root,
            fleet_directory_content_hash,
        }
    }
}

pub(super) fn validate_directory_confirmation_response(
    context: RootDirectoryConfirmationValidationContext<'_>,
    previous: &RootComponentProvisioningStatusResponse,
    response: &RootComponentProvisioningStatusResponse,
    recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let batch = context
        .operation
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == context.root)
        .ok_or_else(|| receipt_invariant("Directory confirmation root has no planned batch"))?;
    let expected_authority =
        RootDirectoryConfirmationAuthority::expected(context.operation, context.root, previous);
    if RootDirectoryConfirmationAuthority::observed(response) != expected_authority {
        return Err(InternalError::conflict());
    }
    if response.published_component_count < previous.published_component_count
        || response.published_component_count > response.component_count
    {
        return Err(InternalError::conflict());
    }
    let publication = response
        .publication
        .as_ref()
        .ok_or_else(InternalError::conflict)?;
    if &publication.fleet_registry != context.published_registry
        || publication.fleet_directory_content_hash != context.fleet_directory_content_hash
    {
        return Err(InternalError::conflict());
    }
    validate_root_publication_evidence(context.operation, batch, response, publication)?;
    match response.phase {
        RootComponentProvisioningPhase::Provisioned => {
            if response.published_at_ns.is_some()
                || response.receipt_content_hash != previous.receipt_content_hash
            {
                return Err(InternalError::conflict());
            }
        }
        RootComponentProvisioningPhase::Published => {
            let result = response.result.as_ref().ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks provisioned result")
            })?;
            let provisioned_at_ns = response.provisioned_at_ns.ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks provisioning time")
            })?;
            let published_at_ns = response.published_at_ns.ok_or_else(|| {
                receipt_invariant("Published Directory confirmation lacks publication time")
            })?;
            if response.published_component_count != response.component_count
                || published_at_ns < provisioned_at_ns
                || recorded_at_ns < published_at_ns
            {
                return Err(InternalError::conflict());
            }
            let expected = RootComponentProvisioningReceiptOps::published_content_hash(
                RootComponentProvisioningPublishedReceiptAuthority {
                    operation_id: context.operation.operation_id,
                    plan_hash: context.operation.plan_hash,
                    configuration_digest: context.operation.plan.configuration_digest,
                    root: &batch.root,
                    result,
                    publication,
                    accepted_at_ns: response.accepted_at_ns,
                    provisioned_at_ns,
                    published_at_ns,
                },
            )?;
            if response.receipt_content_hash != expected {
                return Err(InternalError::conflict());
            }
        }
        _ => return Err(InternalError::conflict()),
    }
    Ok(())
}

#[derive(Eq, PartialEq)]
struct RootDirectoryConfirmationCounts {
    placements: u32,
    components: u32,
    reserved: u32,
    claimed: u32,
    installed: u32,
    registry_committed: u32,
}

#[derive(Eq, PartialEq)]
struct RootDirectoryConfirmationAuthority<'a> {
    operation_id: [u8; 32],
    plan_hash: [u8; 32],
    configuration_digest: &'a ComponentDeploymentConfigurationDigest,
    fleet_registry: &'a FleetRegistryVersion,
    fleet_subnet_root: Principal,
    counts: RootDirectoryConfirmationCounts,
    result: &'a Option<canic_core::dto::component_provisioning::RootComponentProvisioningResult>,
    accepted_at_ns: u64,
    provisioned_at_ns: Option<u64>,
}

impl<'a> RootDirectoryConfirmationAuthority<'a> {
    const fn expected(
        record: &'a FleetComponentProvisioningRecord,
        root: Principal,
        previous: &'a RootComponentProvisioningStatusResponse,
    ) -> Self {
        Self {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: &record.plan.configuration_digest,
            fleet_registry: &record.plan.fleet_registry,
            fleet_subnet_root: root,
            counts: RootDirectoryConfirmationCounts::from_response(previous),
            result: &previous.result,
            accepted_at_ns: previous.accepted_at_ns,
            provisioned_at_ns: previous.provisioned_at_ns,
        }
    }

    const fn observed(response: &'a RootComponentProvisioningStatusResponse) -> Self {
        Self {
            operation_id: response.operation_id,
            plan_hash: response.plan_hash,
            configuration_digest: &response.configuration_digest,
            fleet_registry: &response.fleet_registry,
            fleet_subnet_root: response.fleet_subnet_root,
            counts: RootDirectoryConfirmationCounts::from_response(response),
            result: &response.result,
            accepted_at_ns: response.accepted_at_ns,
            provisioned_at_ns: response.provisioned_at_ns,
        }
    }
}

impl RootDirectoryConfirmationCounts {
    const fn from_response(response: &RootComponentProvisioningStatusResponse) -> Self {
        Self {
            placements: response.placement_count,
            components: response.component_count,
            reserved: response.reserved_component_count,
            claimed: response.claimed_component_count,
            installed: response.installed_component_count,
            registry_committed: response.registry_committed_component_count,
        }
    }
}

fn validate_root_publication_evidence(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    response: &RootComponentProvisioningStatusResponse,
    publication: &canic_core::dto::component_provisioning::RootComponentPublicationEvidence,
) -> Result<(), InternalError> {
    let result = response
        .result
        .as_ref()
        .ok_or_else(|| receipt_invariant("Directory confirmation lacks its provisioned result"))?;
    if publication.component_directories.len()
        != usize::try_from(response.published_component_count)
            .map_err(|_| receipt_invariant("published Component count exceeds usize"))?
        || publication.component_group_directories.len() != result.placements.len()
    {
        return Err(InternalError::conflict());
    }
    for (member, evidence) in result
        .placements
        .iter()
        .flat_map(|placement| &placement.members)
        .zip(&publication.component_directories)
    {
        if evidence.component != member.binding.component
            || evidence.content_hash != member.component_registry_content_hash
        {
            return Err(InternalError::conflict());
        }
    }
    for (index, (planned, provisioned)) in
        batch.placements.iter().zip(&result.placements).enumerate()
    {
        let evidence = &publication.component_group_directories[index];
        let directory =
            component_group_directory_from_receipt(record, batch, planned, provisioned)?;
        let expected_hash =
            RootComponentProvisioningReceiptOps::component_group_directory_content_hash(
                &directory,
            )?;
        if evidence.group_placement != provisioned.group_placement
            || evidence.content_hash != expected_hash
        {
            return Err(InternalError::conflict());
        }
    }
    Ok(())
}

fn component_group_directory_from_receipt(
    record: &FleetComponentProvisioningRecord,
    batch: &FleetSubnetRootProvisioningBatch,
    planned: &canic_core::dto::component_provisioning::ComponentGroupPlacementPlan,
    provisioned: &canic_core::dto::component_provisioning::RootProvisionedGroupPlacement,
) -> Result<canic_core::dto::component_provisioning::ComponentGroupDirectory, InternalError> {
    let placement_matches = [
        planned.group_placement == provisioned.group_placement,
        planned.component_group == provisioned.component_group,
        planned.entries.len() == provisioned.members.len(),
    ]
    .into_iter()
    .all(|matches| matches);
    if !placement_matches {
        return Err(receipt_invariant(
            "Component Group Directory plan differs from provisioned placement",
        ));
    }
    let members = planned
        .entries
        .iter()
        .zip(&provisioned.members)
        .map(|(entry, member)| {
            if entry.member_path != member.member_path
                || entry.component_spec != member.component_spec
                || entry.purpose != member.purpose
            {
                return Err(receipt_invariant(
                    "Component Group Directory member differs from planned occurrence",
                ));
            }
            Ok(
                canic_core::dto::component_provisioning::ComponentGroupDirectoryMember {
                    member_path: member.member_path.clone(),
                    component_spec: member.component_spec.clone(),
                    purpose: member.purpose.clone(),
                    labels: entry.labels.clone(),
                    binding: member.binding.clone(),
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(
        canic_core::dto::component_provisioning::ComponentGroupDirectory {
            provenance:
                canic_core::dto::component_provisioning::ComponentGroupDirectoryProvenance {
                    authority: batch.root.authority.clone(),
                    fleet_subnet_root: batch.root.fleet_subnet_root,
                    group_placement: provisioned.group_placement.clone(),
                    component_group: provisioned.component_group.clone(),
                    operation_id: record.operation_id,
                    plan_hash: record.plan_hash,
                    placement_receipt_content_hash:
                        RootComponentProvisioningReceiptOps::group_placement_content_hash(
                            record.operation_id,
                            record.plan_hash,
                            &batch.root,
                            provisioned,
                        )?,
                },
            members,
        },
    )
}

pub(super) fn validate_component_provisioning_record(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(record) = &current.component_provisioning else {
        if !current.service_publication_receipts.is_empty() {
            return Err(receipt_invariant(
                "Fleet-service publication receipt lacks its provisioning operation",
            ));
        }
        return Ok(());
    };
    if record.operation_id == [0; 32] {
        return Err(receipt_invariant(
            "Fleet Component provisioning operation ID is zero",
        ));
    }
    if record.plan_hash == [0; 32] {
        return Err(receipt_invariant(
            "Fleet Component provisioning plan hash is zero",
        ));
    }
    if record.plan.operation != FleetComponentProvisioningOperation::FreshInstall {
        return Err(receipt_invariant(
            "Fleet Component provisioning record contains an unavailable operation kind",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    ComponentProvisioningPlanOps::validate_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
    )
    .map_err(|_| {
        receipt_invariant(
            "Fleet Component provisioning plan differs from canonical configuration or Registry authority",
        )
    })?;
    let plan_hash = ComponentProvisioningPlanOps::hash_compiled(
        &current.component_deployment_configuration,
        &source_registry,
        &record.plan,
    )
    .map_err(|_| receipt_invariant("Fleet Component provisioning plan hash cannot be rederived"))?;
    if record.plan_hash != plan_hash {
        return Err(receipt_invariant(
            "Fleet Component provisioning plan hash differs from canonical bytes",
        ));
    }
    validate_component_provisioning_root_failure(record)?;
    validate_component_provisioning_estate_funding_pause(record)?;
    validate_component_provisioning_root_acceptance_state(record)?;
    validate_component_provisioning_root_provision_state(
        &current.component_deployment_configuration,
        &source_registry,
        record,
    )?;
    validate_service_publication_authority(current, record)?;
    validate_component_directory_confirmation_state(current, record)?;
    validate_component_runtime_activation_state(record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

fn validate_component_provisioning_estate_funding_pause(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let Some(funding) = record.estate_funding_required.as_ref() else {
        return Ok(());
    };
    let FleetComponentProvisioningStateRecord::ProvisioningRoots {
        provisions,
        current,
        in_flight: None,
        ..
    } = &record.state
    else {
        return Err(receipt_invariant(
            "Fleet Component estate funding pause is outside Root provisioning",
        ));
    };
    let root_index = u32::try_from(provisions.len())
        .map_err(|_| receipt_invariant("estate funding Root index does not fit u32"))?;
    let expected_root = if let Some(current) = current.as_ref() {
        current.response.fleet_subnet_root
    } else {
        root_batch(record, root_index)?.root.fleet_subnet_root
    };
    let required = funding
        .creation_amount
        .to_u128()
        .checked_add(funding.ledger_fee.to_u128())
        .ok_or_else(InternalError::resource_exhausted)?;
    let creation_amount = funding
        .readiness_floor
        .to_u128()
        .checked_add(funding.execution_margin.to_u128())
        .and_then(|amount| amount.checked_add(funding.management_creation_fee.to_u128()))
        .ok_or_else(InternalError::resource_exhausted)?;
    let authority_is_exact = funding.root == expected_root
        && funding.operation_id != [0; 32]
        && funding.readiness_floor.to_u128() > 0
        && funding.execution_margin.to_u128() > 0;
    let arithmetic_is_exact = funding.creation_amount.to_u128() == creation_amount
        && funding.required.to_u128() == required
        && funding.available < funding.required
        && funding.shortfall.to_u128() == required.saturating_sub(funding.available.to_u128());
    let timing_is_exact = funding.retry_at_ns > 0
        && funding
            .last_attempt_at_ns
            .is_none_or(|attempted_at_ns| attempted_at_ns > 0);
    if !authority_is_exact
        || !arithmetic_is_exact
        || !timing_is_exact
        || record.last_root_failure.is_some()
    {
        return Err(receipt_invariant(
            "Fleet Component estate funding pause is not exactly bound",
        ));
    }
    Ok(())
}

fn validate_component_provisioning_root_failure(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let Some(failure) = record.last_root_failure else {
        return Ok(());
    };
    let root_is_bound = match failure.stage {
        FleetComponentProvisioningRetryStage::RootAcceptance
        | FleetComponentProvisioningRetryStage::RootProvisioning => record
            .plan
            .batches
            .iter()
            .any(|batch| batch.root.fleet_subnet_root == failure.fleet_subnet_root),
        FleetComponentProvisioningRetryStage::DirectoryConfirmation
        | FleetComponentProvisioningRetryStage::RuntimeActivation => record
            .plan
            .directory_confirmation_roots
            .contains(&failure.fleet_subnet_root),
    };
    let planned_at_ns = component_provisioning_root_acceptance_progress(record)?.planned_at_ns;
    if failure.diagnostic_code == 0 || failure.failed_at_ns < planned_at_ns || !root_is_bound {
        return Err(receipt_invariant(
            "Fleet Component provisioning retry failure is outside its bounded Root authority",
        ));
    }
    Ok(())
}

pub(super) fn validate_component_scale_out_progress(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let Some(record) = &current.component_scale_out else {
        return Ok(());
    };
    if !matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    ) {
        return Err(receipt_invariant(
            "Fleet Component scale-out record contains a different operation kind",
        ));
    }
    if !matches!(
        record.state,
        FleetComponentProvisioningStateRecord::Planned { .. }
            | FleetComponentProvisioningStateRecord::AcceptingRoots { .. }
            | FleetComponentProvisioningStateRecord::RootsAccepted { .. }
            | FleetComponentProvisioningStateRecord::ProvisioningRoots { .. }
            | FleetComponentProvisioningStateRecord::ComponentsProvisioned { .. }
            | FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. }
            | FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
            | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
            | FleetComponentProvisioningStateRecord::RuntimesActivated { .. }
    ) {
        return Err(receipt_invariant(
            "Fleet Component scale-out has an invalid runtime-activation state",
        ));
    }
    let source_registry = component_operation_source_registry(current, record)?;
    validate_component_provisioning_root_acceptance_state(record)?;
    validate_component_provisioning_root_provision_state(
        &current.component_deployment_configuration,
        &source_registry,
        record,
    )?;
    validate_service_publication_authority(current, record)?;
    validate_scale_out_service_publication_fence(record)?;
    validate_component_directory_confirmation_state(current, record)?;
    validate_component_runtime_activation_state(record)?;
    component_provisioning_plan_counts(&record.plan)?;
    Ok(())
}

pub(super) fn validate_component_scale_out_receipts(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    if current.component_scale_out_receipts.len() > MAX_FLEET_COMPONENT_PROVISIONING_PLAN_PLACEMENTS
    {
        return Err(receipt_invariant(
            "retired scale-out receipt count exceeds the placement bound",
        ));
    }
    let fresh_operation = current
        .component_provisioning
        .as_ref()
        .map(|record| record.operation_id);
    let active_operation = current
        .component_scale_out
        .as_ref()
        .map(|record| record.operation_id);
    let configuration_digest = current
        .component_deployment_configuration
        .digest()
        .map_err(|_| receipt_invariant("deployment configuration digest cannot be rederived"))?;
    let mut operation_ids = BTreeSet::new();
    let mut previous_completed_at_ns = 0_u64;
    for receipt in &current.component_scale_out_receipts {
        validate_retired_scale_out_identity(
            receipt,
            configuration_digest,
            fresh_operation,
            active_operation,
            operation_ids.insert(receipt.operation_id),
        )?;
        validate_retired_scale_out_content_hash(receipt)?;
        let authority = retired_scale_out_authority(receipt)?;
        validate_retired_scale_out_counts(receipt, authority.placement_count)?;
        validate_retired_scale_out_times(receipt, previous_completed_at_ns)?;
        validate_retired_scale_out_registry(receipt, &current.authority)?;
        validate_retired_scale_out_placements(receipt, &authority)?;
        validate_retired_scale_out_publication(current, receipt)?;
        component_scale_out_receipt_response(receipt)?;
        previous_completed_at_ns = receipt.runtimes_activated_at_ns;
    }
    if let Some(active) = &current.component_scale_out {
        let active = component_provisioning_status_response(active)?;
        if active.planned_at_ns < previous_completed_at_ns {
            return Err(receipt_invariant(
                "active scale-out journal predates retired terminal history",
            ));
        }
    }
    Ok(())
}

struct RetiredScaleOutAuthority<'a> {
    deployment: &'a ComponentGroupDeploymentId,
    previous_placements: u32,
    placement_count: usize,
}

fn retired_scale_out_authority(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<RetiredScaleOutAuthority<'_>, InternalError> {
    let FleetComponentProvisioningOperation::ScaleOut {
        deployment,
        previous_placements,
        requested_placements,
    } = &receipt.operation
    else {
        return Err(receipt_invariant(
            "retired Component operation is not scale-out",
        ));
    };
    let placement_count = requested_placements
        .checked_sub(*previous_placements)
        .filter(|count| *count > 0)
        .ok_or_else(|| receipt_invariant("retired scale-out count is not monotonic"))?;
    Ok(RetiredScaleOutAuthority {
        deployment,
        previous_placements: *previous_placements,
        placement_count: usize::try_from(placement_count)
            .map_err(|_| receipt_invariant("retired scale-out count does not fit usize"))?,
    })
}

fn validate_retired_scale_out_identity(
    receipt: &FleetComponentScaleOutReceiptRecord,
    configuration_digest: ComponentDeploymentConfigurationDigest,
    fresh_operation: Option<[u8; 32]>,
    active_operation: Option<[u8; 32]>,
    operation_is_unique: bool,
) -> Result<(), InternalError> {
    let identity_facts = [
        receipt.operation_id != [0; 32],
        receipt.plan_hash != [0; 32],
        fresh_operation != Some(receipt.operation_id),
        active_operation != Some(receipt.operation_id),
        operation_is_unique,
        receipt.configuration_digest == configuration_digest,
    ];
    if identity_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid or reused operation authority",
        ))
    }
}

fn validate_retired_scale_out_content_hash(
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<(), InternalError> {
    if receipt.receipt_content_hash == [0; 32]
        || receipt.receipt_content_hash != component_scale_out_receipt_content_hash(receipt)?
    {
        return Err(receipt_invariant(
            "retired scale-out receipt content hash is invalid",
        ));
    }
    Ok(())
}

fn validate_retired_scale_out_counts(
    receipt: &FleetComponentScaleOutReceiptRecord,
    placement_count: usize,
) -> Result<(), InternalError> {
    let root_batch_count = usize::try_from(receipt.root_batch_count)
        .map_err(|_| receipt_invariant("retired root count does not fit usize"))?;
    let confirmation_root_count = usize::try_from(receipt.directory_confirmation_root_count)
        .map_err(|_| receipt_invariant("retired confirmation count does not fit usize"))?;
    let component_count = usize::try_from(receipt.component_count)
        .map_err(|_| receipt_invariant("retired Component count does not fit usize"))?;
    let count_facts = [
        receipt.placements.len() == placement_count,
        root_batch_count > 0,
        root_batch_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_BATCHES,
        confirmation_root_count >= root_batch_count,
        confirmation_root_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_CONFIRMATION_ROOTS,
        component_count >= placement_count,
        component_count <= MAX_FLEET_COMPONENT_PROVISIONING_PLAN_ENTRIES,
    ];
    if count_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid bounded counts",
        ))
    }
}

fn validate_retired_scale_out_times(
    receipt: &FleetComponentScaleOutReceiptRecord,
    previous_completed_at_ns: u64,
) -> Result<(), InternalError> {
    let times = [
        receipt.planned_at_ns,
        receipt.roots_accepted_at_ns,
        receipt.components_provisioned_at_ns,
        receipt.service_topology_published_at_ns,
        receipt.directories_confirmed_at_ns,
        receipt.runtimes_activated_at_ns,
    ];
    let time_facts = [
        receipt.planned_at_ns >= previous_completed_at_ns,
        times[0] > 0,
        times.windows(2).all(|pair| pair[0] <= pair[1]),
    ];
    if time_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid terminal ordering",
        ))
    }
}

fn validate_retired_scale_out_registry(
    receipt: &FleetComponentScaleOutReceiptRecord,
    authority: &FleetRegistryAuthority,
) -> Result<(), InternalError> {
    let registry_facts = [
        &receipt.fleet_registry.authority == authority,
        &receipt.published_fleet_registry.authority == authority,
        receipt.published_fleet_registry.revision >= receipt.fleet_registry.revision,
    ];
    if registry_facts.into_iter().all(|fact| fact) {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt has invalid Fleet Registry authority",
        ))
    }
}

fn validate_retired_scale_out_placements(
    receipt: &FleetComponentScaleOutReceiptRecord,
    authority: &RetiredScaleOutAuthority<'_>,
) -> Result<(), InternalError> {
    let mut selected_root_receipts = BTreeSet::new();
    for (offset, placement) in receipt.placements.iter().enumerate() {
        let offset = u32::try_from(offset)
            .map_err(|_| receipt_invariant("retired placement offset does not fit u32"))?;
        let ordinal = authority
            .previous_placements
            .checked_add(offset)
            .ok_or_else(|| receipt_invariant("retired placement ordinal overflowed"))?;
        let placement_facts = [
            &placement.placement.deployment == authority.deployment,
            placement.placement.ordinal == ordinal,
            placement.operation_id == receipt.operation_id,
            placement.plan_hash == receipt.plan_hash,
            placement.root_receipt_content_hash != [0; 32],
        ];
        if !placement_facts.into_iter().all(|fact| fact) {
            return Err(receipt_invariant(
                "retired scale-out placement authority is invalid",
            ));
        }
        selected_root_receipts.insert((
            placement.fleet_subnet_root,
            placement.root_receipt_content_hash,
        ));
    }
    if selected_root_receipts.len()
        == usize::try_from(receipt.root_batch_count)
            .map_err(|_| receipt_invariant("retired root count does not fit usize"))?
    {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out receipt lacks exact selected-root evidence",
        ))
    }
}

fn validate_retired_scale_out_publication(
    current: &FleetCoordinatorRegistryRecord,
    receipt: &FleetComponentScaleOutReceiptRecord,
) -> Result<(), InternalError> {
    let publication = service_publication_receipt_for_operation(current, receipt.operation_id)?
        .ok_or_else(|| receipt_invariant("retired scale-out lacks publication authority"))?;
    let actual = (
        publication.operation_id,
        publication.plan_hash,
        publication.configuration_digest,
        &publication.previous_version,
        &publication.version,
    );
    let expected = (
        receipt.operation_id,
        receipt.plan_hash,
        receipt.configuration_digest,
        &receipt.fleet_registry,
        &receipt.published_fleet_registry,
    );
    if actual == expected {
        Ok(())
    } else {
        Err(receipt_invariant(
            "retired scale-out publication authority is invalid",
        ))
    }
}

pub(super) fn validate_service_publication_receipt_owners(
    current: &FleetCoordinatorRegistryRecord,
) -> Result<(), InternalError> {
    let mut operation_ids = current
        .component_scale_out_receipts
        .iter()
        .map(|receipt| receipt.operation_id)
        .collect::<BTreeSet<_>>();
    operation_ids.extend(
        [
            current.component_provisioning.as_ref(),
            current.component_scale_out.as_ref(),
        ]
        .into_iter()
        .flatten()
        .map(|record| record.operation_id),
    );
    for receipt in &current.service_publication_receipts {
        if !operation_ids.contains(&receipt.operation_id) {
            return Err(receipt_invariant(
                "Fleet-service publication receipt lacks its provisioning operation",
            ));
        }
    }
    Ok(())
}

fn validate_scale_out_service_publication_fence(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = component_provisioning_root_provision_progress(record)?;
    let current_root_crossed_terminal_fence = progress
        .current_response
        .as_ref()
        .is_some_and(|response| response.phase != RootComponentProvisioningPhase::Accepted);
    if current_root_crossed_terminal_fence {
        return Err(receipt_invariant(
            "Fleet Component scale-out current root crossed its terminal provisioning fence",
        ));
    }
    Ok(())
}

fn validate_component_directory_confirmation_state(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ServiceTopologyPublished { .. }
        | FleetComponentProvisioningStateRecord::ConfirmingDirectories { .. }
        | FleetComponentProvisioningStateRecord::DirectoriesConfirmed { .. }
        | FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            component_directory_confirmation_progress(record)?
        }
        _ => return Ok(()),
    };
    let selected_root_count = u32::try_from(record.plan.batches.len())
        .map_err(|_| receipt_invariant("root batch count does not fit u32"))?;
    let scale_out = matches!(
        record.plan.operation,
        FleetComponentProvisioningOperation::ScaleOut { .. }
    );
    let root_count_is_valid = if scale_out {
        selected_root_count <= progress.confirmation_root_count
            && record.plan.batches.iter().all(|batch| {
                record
                    .plan
                    .directory_confirmation_roots
                    .contains(&batch.root.fleet_subnet_root)
            })
    } else {
        selected_root_count == progress.confirmation_root_count
    };
    if !root_count_is_valid || progress.confirmed_root_count > progress.confirmation_root_count {
        return Err(receipt_invariant(
            "Directory confirmation roots differ from the protected barrier",
        ));
    }
    let mut previous_recorded_at_ns = validate_completed_directory_confirmations(
        coordinator,
        record,
        &progress,
        progress.service_topology_published_at_ns,
    )?;
    previous_recorded_at_ns = validate_current_directory_confirmation(
        coordinator,
        record,
        &progress,
        previous_recorded_at_ns,
    )?;
    validate_directory_confirmation_intent(record, &progress, previous_recorded_at_ns)?;
    validate_terminal_directory_confirmation(record, &progress, previous_recorded_at_ns)
}

fn validate_component_runtime_activation_state(
    record: &FleetComponentProvisioningRecord,
) -> Result<(), InternalError> {
    let progress = match &record.state {
        FleetComponentProvisioningStateRecord::ActivatingRuntimes { .. }
        | FleetComponentProvisioningStateRecord::RuntimesActivated { .. } => {
            component_runtime_activation_progress(record)?
        }
        _ => return Ok(()),
    };
    if progress.activation_root_count
        != u32::try_from(record.plan.batches.len())
            .map_err(|_| receipt_invariant("runtime activation root count does not fit u32"))?
        || progress.activated_root_count > progress.activation_root_count
    {
        return Err(receipt_invariant(
            "runtime activation roots differ from selected root batches",
        ));
    }
    let mut previous_recorded_at_ns = progress.directories_confirmed_at_ns;
    for (index, activation) in progress.activations.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("runtime activation root index does not fit u32"))?;
        validate_stored_runtime_activation(record, &progress, root_index, activation, true)?;
        if activation.started_at_ns < previous_recorded_at_ns
            || activation.recorded_at_ns < activation.started_at_ns
        {
            return Err(receipt_invariant(
                "runtime activation observation time evidence is invalid",
            ));
        }
        previous_recorded_at_ns = activation.recorded_at_ns;
    }
    if let Some(current) = &progress.current {
        validate_stored_runtime_activation(
            record,
            &progress,
            progress.activated_root_count,
            current,
            false,
        )?;
        if current.started_at_ns < previous_recorded_at_ns
            || current.recorded_at_ns < current.started_at_ns
        {
            return Err(receipt_invariant(
                "current runtime activation observation time evidence is invalid",
            ));
        }
        previous_recorded_at_ns = current.recorded_at_ns;
    }
    validate_runtime_activation_intent(record, &progress, previous_recorded_at_ns)?;
    validate_terminal_runtime_activation_state(&progress, previous_recorded_at_ns)
}

fn validate_stored_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentRuntimeActivationProgress,
    root_index: u32,
    activation: &FleetComponentRuntimeActivationRecord,
    terminal: bool,
) -> Result<(), InternalError> {
    let publication = root_publication_response(record, progress, root_index)?;
    let expected_progress = FleetComponentActivationRootProgress {
        fleet_subnet_root: publication.fleet_subnet_root,
        component_count: publication.component_count,
        activated_component_count: if terminal {
            publication.component_count
        } else {
            activation.progress.activated_component_count
        },
        root_runtime_active: terminal,
    };
    let activation_started_at_ns = activation
        .activation_started_at_ns
        .ok_or_else(|| receipt_invariant("stored runtime activation lacks its root start time"))?;
    let published_at_ns = publication
        .published_at_ns
        .ok_or_else(|| receipt_invariant("stored root publication lacks completion time"))?;
    let component_cursor_is_bounded = terminal
        || (activation.progress.activated_component_count > 0
            && activation.progress.activated_component_count <= publication.component_count);
    if activation.progress != expected_progress
        || !component_cursor_is_bounded
        || activation_started_at_ns < published_at_ns
        || activation.recorded_at_ns < activation_started_at_ns
    {
        return Err(receipt_invariant(
            "stored runtime activation progress or time authority is invalid",
        ));
    }
    if terminal {
        validate_stored_terminal_runtime_activation(
            record,
            root_index,
            publication,
            activation,
            activation_started_at_ns,
        )
    } else if activation.activation.is_some()
        || activation.runtimes_activated_at_ns.is_some()
        || activation.receipt_content_hash != publication.receipt_content_hash
    {
        Err(receipt_invariant(
            "stored in-progress runtime activation changed publication authority",
        ))
    } else {
        Ok(())
    }
}

fn validate_stored_terminal_runtime_activation(
    record: &FleetComponentProvisioningRecord,
    root_index: u32,
    publication: &RootComponentProvisioningStatusResponse,
    stored: &FleetComponentRuntimeActivationRecord,
    activation_started_at_ns: u64,
) -> Result<(), InternalError> {
    let activation = stored
        .activation
        .ok_or_else(|| receipt_invariant("stored terminal runtime activation lacks evidence"))?;
    let runtimes_activated_at_ns = stored.runtimes_activated_at_ns.ok_or_else(|| {
        receipt_invariant("stored terminal runtime activation lacks completion time")
    })?;
    let identity_is_exact = activation.component_count == publication.component_count
        && activation.fleet_activation_operation_id != [0; 32]
        && activation.initial_inventory_hash != [0; 32];
    let timing_is_exact = terminal_root_activation_timing_is_valid(
        &record.plan.operation,
        activation.root_activated_at_ns,
        publication.accepted_at_ns,
        activation_started_at_ns,
        runtimes_activated_at_ns,
    );
    let observation_is_exact = stored.recorded_at_ns >= runtimes_activated_at_ns;
    let evidence_is_exact = identity_is_exact && timing_is_exact && observation_is_exact;
    if !evidence_is_exact {
        return Err(receipt_invariant(
            "stored terminal runtime activation evidence is invalid",
        ));
    }
    let batch = root_batch(record, root_index)?;
    let expected = RootComponentProvisioningReceiptOps::runtimes_active_content_hash(
        RootComponentProvisioningRuntimesActiveReceiptAuthority {
            operation_id: record.operation_id,
            plan_hash: record.plan_hash,
            configuration_digest: record.plan.configuration_digest,
            root: &batch.root,
            published_receipt_content_hash: publication.receipt_content_hash,
            activation,
            activation_started_at_ns,
            runtimes_activated_at_ns,
        },
    )?;
    if stored.receipt_content_hash != expected {
        return Err(receipt_invariant(
            "stored terminal runtime activation receipt hash is invalid",
        ));
    }
    Ok(())
}

fn validate_runtime_activation_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentRuntimeActivationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    let root = activation_root(record, progress.activated_root_count)?;
    let current = progress.current.map_or_else(
        || {
            root_publication_response(record, progress, progress.activated_root_count)
                .map(root_activation_progress)
        },
        |current| Ok(current.progress),
    )?;
    let intent_is_exact = [
        intent.root_index == progress.activated_root_count,
        intent.fleet_subnet_root == root,
        intent.request.operation_id == record.operation_id,
        intent.request.plan_hash == record.plan_hash,
        intent.request.expected_activated_component_count == current.activated_component_count,
        intent.request.expected_root_runtime_active == current.root_runtime_active,
        intent.started_at_ns >= previous_recorded_at_ns,
    ]
    .into_iter()
    .all(|matches| matches);
    if !intent_is_exact {
        return Err(receipt_invariant(
            "runtime activation pre-call intent is invalid",
        ));
    }
    Ok(())
}

fn validate_terminal_runtime_activation_state(
    progress: &FleetComponentRuntimeActivationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.complete {
        let completed_at_ns = progress.runtimes_activated_at_ns.ok_or_else(|| {
            receipt_invariant("terminal runtime activation lacks completion time")
        })?;
        if progress.activated_root_count != progress.activation_root_count
            || progress.current.is_some()
            || progress.in_flight.is_some()
            || completed_at_ns < previous_recorded_at_ns
        {
            return Err(receipt_invariant(
                "terminal runtime activation evidence is incomplete",
            ));
        }
    } else if progress.activated_root_count >= progress.activation_root_count {
        return Err(receipt_invariant(
            "runtime activation remained nonterminal after every selected root",
        ));
    }
    Ok(())
}

fn validate_completed_directory_confirmations(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    mut previous_recorded_at_ns: u64,
) -> Result<u64, InternalError> {
    for (index, confirmation) in progress.confirmations.iter().enumerate() {
        let root_index = u32::try_from(index)
            .map_err(|_| receipt_invariant("Directory confirmation index does not fit u32"))?;
        let root = confirmation_root(record, root_index)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            validate_stored_scale_out_confirmation(
                coordinator,
                record,
                progress,
                root,
                confirmation,
                true,
            )?;
        } else {
            let response = fresh_confirmation_response(confirmation)?;
            let previous = root_provisioned_response(progress, root_index)?;
            let fleet_directory_content_hash = expected_fleet_directory_content_hash(
                coordinator,
                &progress.published_fleet_registry,
                root,
            )?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    fleet_directory_content_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(confirmation),
            )
            .map_err(|_| receipt_invariant("stored Directory confirmation receipt is invalid"))?;
            if response.phase != RootComponentProvisioningPhase::Published {
                return Err(receipt_invariant(
                    "stored fresh Directory confirmation is not terminal",
                ));
            }
        }
        let started_at_ns = confirmation_started_at_ns(confirmation);
        let recorded_at_ns = confirmation_recorded_at_ns(confirmation);
        if started_at_ns < previous_recorded_at_ns || recorded_at_ns < started_at_ns {
            return Err(receipt_invariant(
                "stored Directory confirmation time or terminal phase is invalid",
            ));
        }
        previous_recorded_at_ns = recorded_at_ns;
    }
    Ok(previous_recorded_at_ns)
}

fn validate_current_directory_confirmation(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    mut previous_recorded_at_ns: u64,
) -> Result<u64, InternalError> {
    if let Some(current) = &progress.current {
        let root = confirmation_root(record, progress.confirmed_root_count)?;
        if matches!(
            record.plan.operation,
            FleetComponentProvisioningOperation::ScaleOut { .. }
        ) {
            validate_stored_scale_out_confirmation(
                coordinator,
                record,
                progress,
                root,
                current,
                false,
            )?;
        } else {
            let response = fresh_confirmation_response(current)?;
            let previous = root_provisioned_response(progress, progress.confirmed_root_count)?;
            let fleet_directory_content_hash = expected_fleet_directory_content_hash(
                coordinator,
                &progress.published_fleet_registry,
                root,
            )?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    fleet_directory_content_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(current),
            )
            .map_err(|_| {
                receipt_invariant("stored in-progress Directory confirmation is invalid")
            })?;
            if response.phase != RootComponentProvisioningPhase::Provisioned {
                return Err(receipt_invariant(
                    "stored fresh Directory confirmation crossed its terminal boundary",
                ));
            }
        }
        let started_at_ns = confirmation_started_at_ns(current);
        let recorded_at_ns = confirmation_recorded_at_ns(current);
        if started_at_ns < previous_recorded_at_ns || recorded_at_ns < started_at_ns {
            return Err(receipt_invariant(
                "in-progress Directory confirmation time or phase is invalid",
            ));
        }
        previous_recorded_at_ns = recorded_at_ns;
    }
    Ok(previous_recorded_at_ns)
}

fn validate_stored_scale_out_confirmation(
    coordinator: &FleetCoordinatorRegistryRecord,
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    root: Principal,
    confirmation: &FleetComponentDirectoryConfirmationRecord,
    terminal: bool,
) -> Result<(), InternalError> {
    let (synchronization, publication) = scale_out_confirmation_progress(confirmation)?;
    let expected_directory_hash = expected_fleet_directory_content_hash(
        coordinator,
        &progress.published_fleet_registry,
        root,
    )?;
    let synchronization_authority_is_exact = [
        synchronization.operation_id == record.operation_id,
        synchronization.plan_hash == record.plan_hash,
        synchronization.source_fleet_registry == record.plan.fleet_registry,
        synchronization.published_fleet_registry == progress.published_fleet_registry,
        synchronization.fleet_subnet_root == root,
        synchronization.fleet_directory_content_hash == expected_directory_hash,
        synchronization.synchronized_component_count <= synchronization.affected_component_count,
    ]
    .into_iter()
    .all(|matches| matches);
    if !synchronization_authority_is_exact {
        return Err(receipt_invariant(
            "stored scale-out Directory synchronization authority is invalid",
        ));
    }
    let synchronization_evidence_is_exact = if synchronization.complete {
        [
            synchronization.synchronized_component_count
                == synchronization.affected_component_count,
            synchronization.synchronized_at_ns.is_some_and(|time| {
                time >= confirmation_started_at_ns(confirmation)
                    && confirmation_recorded_at_ns(confirmation) >= time
            }),
            synchronization.receipt_content_hash
                == RootComponentProvisioningReceiptOps::directory_synchronization_content_hash(
                    synchronization,
                )?,
        ]
        .into_iter()
        .all(|matches| matches)
    } else {
        [
            synchronization.synchronized_component_count < synchronization.affected_component_count,
            synchronization.synchronized_at_ns.is_none(),
            synchronization.receipt_content_hash == [0; 32],
        ]
        .into_iter()
        .all(|matches| matches)
    };
    if !synchronization_evidence_is_exact {
        return Err(receipt_invariant(
            "stored scale-out Directory synchronization evidence is invalid",
        ));
    }
    let selected_batch = record
        .plan
        .batches
        .iter()
        .find(|batch| batch.root.fleet_subnet_root == root);
    match (selected_batch, publication, terminal) {
        (None, None, true) if synchronization.complete => Ok(()),
        (None, None, false) if !synchronization.complete => Ok(()),
        (Some(_), None, false) => Ok(()),
        (Some(_), Some(response), expected_terminal) => {
            if !synchronization.complete {
                return Err(receipt_invariant(
                    "stored scale-out publication preceded Directory synchronization",
                ));
            }
            let previous = selected_root_provisioned_response(record, progress, root)?;
            validate_directory_confirmation_response(
                RootDirectoryConfirmationValidationContext::new(
                    record,
                    &progress.published_fleet_registry,
                    root,
                    expected_directory_hash,
                ),
                previous,
                response,
                confirmation_recorded_at_ns(confirmation),
            )
            .map_err(|_| receipt_invariant("stored scale-out Directory publication is invalid"))?;
            let is_terminal = response.phase == RootComponentProvisioningPhase::Published;
            if is_terminal != expected_terminal {
                return Err(receipt_invariant(
                    "stored scale-out Directory publication phase is invalid",
                ));
            }
            Ok(())
        }
        _ => Err(receipt_invariant(
            "stored scale-out Directory confirmation has invalid root evidence",
        )),
    }
}

fn validate_directory_confirmation_intent(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    let Some(intent) = &progress.in_flight else {
        return Ok(());
    };
    let root = confirmation_root(record, progress.confirmed_root_count)?;
    let intent_is_exact = match intent {
        FleetComponentDirectoryConfirmationIntentRecord::FreshPublication {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let previous = progress
                .current
                .as_ref()
                .map(fresh_confirmation_response)
                .transpose()?
                .map_or_else(
                    || root_provisioned_response(progress, progress.confirmed_root_count),
                    Ok,
                )?;
            [
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_published_component_count == previous.published_component_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutSynchronization {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let expected_count = progress
                .current
                .as_ref()
                .map(scale_out_confirmation_progress)
                .transpose()?
                .map_or(0, |(synchronization, _)| {
                    synchronization.synchronized_component_count
                });
            [
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.source_fleet_registry == record.plan.fleet_registry,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_synchronized_component_count == expected_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
        FleetComponentDirectoryConfirmationIntentRecord::ScaleOutPublication {
            root_index,
            fleet_subnet_root,
            request,
            started_at_ns,
        } => {
            let current = progress.current.as_ref().ok_or_else(|| {
                receipt_invariant("scale-out publication intent lacks synchronization evidence")
            })?;
            let (synchronization, publication) = scale_out_confirmation_progress(current)?;
            let previous = publication.map_or_else(
                || selected_root_provisioned_response(record, progress, root),
                Ok,
            )?;
            [
                synchronization.complete,
                *root_index == progress.confirmed_root_count,
                *fleet_subnet_root == root,
                request.operation_id == record.operation_id,
                request.plan_hash == record.plan_hash,
                request.published_fleet_registry == progress.published_fleet_registry,
                request.expected_published_component_count == previous.published_component_count,
                *started_at_ns >= previous_recorded_at_ns,
            ]
            .into_iter()
            .all(|matches| matches)
        }
    };
    if !intent_is_exact {
        return Err(receipt_invariant(
            "Directory confirmation pre-call intent is invalid",
        ));
    }
    Ok(())
}

fn validate_terminal_directory_confirmation(
    record: &FleetComponentProvisioningRecord,
    progress: &FleetComponentDirectoryConfirmationProgress,
    previous_recorded_at_ns: u64,
) -> Result<(), InternalError> {
    if progress.complete {
        let directories_confirmed_at_ns = match &record.state {
            FleetComponentProvisioningStateRecord::DirectoriesConfirmed {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::ActivatingRuntimes {
                directories_confirmed_at_ns,
                ..
            }
            | FleetComponentProvisioningStateRecord::RuntimesActivated {
                directories_confirmed_at_ns,
                ..
            } => *directories_confirmed_at_ns,
            _ => unreachable!("complete Directory progress has terminal state"),
        };
        if progress.confirmed_root_count != progress.confirmation_root_count
            || directories_confirmed_at_ns < previous_recorded_at_ns
        {
            return Err(receipt_invariant(
                "terminal Directory confirmation evidence is incomplete",
            ));
        }
    }
    Ok(())
}
