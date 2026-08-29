use super::{
    AllocationDefinition, AllocationOwner, BuiltInRoleKind, CanicFeatureEffect, CanicFeatureKey,
    MemoryId, RoleCapabilityKey, RoleContractFinding, RoleContractInput, RoleContractResolution,
    RoleContractSource, SelectionProvenance, StateAllocationKey, allocation,
    built_in_role_capabilities,
    catalog::{self, default_features, implied_features},
    derive_role_capabilities, resolve_effective_features, resolve_role_contract,
};
use crate::{
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, IndexConfig,
        LocalApplicationAuthorizationConfig, ScalingConfig, ShardingConfig, TopupPolicy,
    },
    ids::CanisterRole,
    test::config::ConfigTestBuilder,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[test]
fn catalog_matches_canic_cargo_features() {
    let canic_manifest = read_manifest("../canic/Cargo.toml");
    let core_manifest = read_manifest("Cargo.toml");
    let canic_features = feature_table(&canic_manifest);
    let core_features = feature_table(&core_manifest);

    let cargo_public_features = canic_features
        .keys()
        .filter(|name| {
            name.as_str() != "default" && !catalog::is_non_role_public_feature(name.as_str())
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let catalog_public_features = catalog::feature_definitions()
        .iter()
        .map(|definition| definition.cargo_name.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_public_features, cargo_public_features);

    let cargo_non_role_features = canic_features
        .keys()
        .filter(|name| catalog::is_non_role_public_feature(name.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    let catalog_non_role_features = catalog::non_role_public_features()
        .iter()
        .map(|name| (*name).to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_non_role_features, cargo_non_role_features);

    let cargo_defaults = feature_members(canic_features, "default")
        .into_iter()
        .collect::<BTreeSet<_>>();
    let catalog_defaults = default_features()
        .iter()
        .map(|feature| feature.cargo_name().to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_defaults, cargo_defaults);

    let cargo_implications =
        cargo_public_implications(canic_features, core_features, &cargo_public_features);
    let catalog_implications = CanicFeatureKey::ALL
        .iter()
        .flat_map(|feature| {
            implied_features(*feature).map(|implied| {
                (
                    feature.cargo_name().to_string(),
                    implied.cargo_name().to_string(),
                )
            })
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_implications, cargo_implications);
}

#[test]
fn catalog_is_valid_and_classifies_every_public_feature() {
    catalog::validate_catalog().expect("canonical role-contract catalog should be valid");

    for feature in CanicFeatureKey::ALL {
        assert!(matches!(
            feature.effect(),
            CanicFeatureEffect::NoState | CanicFeatureEffect::StateBearing
        ));
    }
}

#[test]
fn canonical_allocations_match_the_active_memory_map() {
    allocation::validate_canonical_allocations()
        .expect("canonical allocation definitions should be valid");

    let actual = allocation::allocation_definitions()
        .iter()
        .map(|definition| {
            (
                definition.key,
                definition
                    .memory_ids
                    .iter()
                    .map(|memory_id| memory_id.get())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = BTreeMap::from([
        (StateAllocationKey::CoreRuntimeChildren, vec![30]),
        (StateAllocationKey::CoreRuntimeBindings, vec![31]),
        (StateAllocationKey::CoreFleetState, vec![32]),
        (StateAllocationKey::CoreFleetActivation, vec![33]),
        (StateAllocationKey::CoreAuthState, vec![34]),
        (StateAllocationKey::CoreReplayReceipts, vec![35]),
        (StateAllocationKey::CoreCycles, vec![36, 37, 38]),
        (StateAllocationKey::CoreCyclesIcpRefillRecords, vec![39]),
        (StateAllocationKey::CoreRuntimeLog, vec![40]),
        (StateAllocationKey::CoreIntent, vec![41, 42, 43, 44, 45, 46]),
        (StateAllocationKey::CoreApplicationReceipts, vec![47, 48]),
        (StateAllocationKey::CorePlacementAcknowledgement, vec![49]),
        (StateAllocationKey::PlacementScalingRegistry, vec![50]),
        (StateAllocationKey::PlacementIndexRegistry, vec![51]),
        (StateAllocationKey::ShardingRegistry, vec![52]),
        (StateAllocationKey::ShardingAssignments, vec![53]),
        (StateAllocationKey::ShardingActiveSet, vec![54]),
        (StateAllocationKey::BlobStorageRoots, vec![55]),
        (StateAllocationKey::BlobStoragePendingDeletions, vec![56]),
        (StateAllocationKey::BlobStorageGatewayPrincipals, vec![57]),
        (StateAllocationKey::BlobStorageBilling, vec![58]),
        (StateAllocationKey::CoreAuthorityRestoreFence, vec![59]),
        (StateAllocationKey::CoreAsyncJobRecovery, vec![60]),
        (StateAllocationKey::CoreFleetAdmissionProjection, vec![61]),
        (StateAllocationKey::FleetCoordinatorFunding, vec![62]),
        (StateAllocationKey::RootFunding, vec![63]),
        (StateAllocationKey::FleetCoordinatorAdmission, vec![64]),
        (StateAllocationKey::RootAdmission, vec![65]),
        (StateAllocationKey::TemplateManifests, vec![10]),
        (StateAllocationKey::TemplateChunkSets, vec![11]),
        (StateAllocationKey::TemplateChunkRefs, vec![12]),
        (StateAllocationKey::TemplateChunkPayloads, vec![13]),
        (StateAllocationKey::WasmStoreGcState, vec![14]),
        (StateAllocationKey::FleetCoordinatorRegistry, vec![15]),
        (StateAllocationKey::RootWasmStoreState, vec![16]),
        (StateAllocationKey::RootFleetRegistryMirror, vec![17]),
        (
            StateAllocationKey::RootComponentRegistry,
            vec![18, 19, 20, 21, 22, 23],
        ),
        (StateAllocationKey::RootCanisterPool, vec![24, 25, 26]),
        (
            StateAllocationKey::RootComponentProvisioning,
            vec![27, 28, 29],
        ),
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn canonical_allocations_form_packed_owner_ledgers() {
    let ids = |owner| {
        let mut ids = allocation::allocation_definitions()
            .iter()
            .filter(|definition| definition.owner == owner)
            .flat_map(|definition| definition.memory_ids)
            .map(|memory_id| memory_id.get())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids
    };

    assert_eq!(
        ids(AllocationOwner::CanicControlPlane),
        (allocation::CANIC_CONTROL_PLANE_MIN_ID
            ..=allocation::memory::control_plane::ROOT_COMPONENT_PROVISIONING_STATE_ID)
            .chain(std::iter::once(
                allocation::memory::control_plane::FLEET_COORDINATOR_FUNDING_ID,
            ))
            .chain(std::iter::once(
                allocation::memory::control_plane::ROOT_FUNDING_ID,
            ))
            .chain(std::iter::once(
                allocation::memory::control_plane::FLEET_COORDINATOR_ADMISSION_ID,
            ))
            .chain(std::iter::once(
                allocation::memory::control_plane::ROOT_ADMISSION_ID,
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        ids(AllocationOwner::CanicCore),
        (allocation::CANIC_CORE_MIN_ID
            ..=allocation::memory::fleet_admission_projection::FLEET_ADMISSION_PROJECTION_ID)
            .collect::<Vec<_>>()
    );
}

#[test]
fn distinct_allocation_keys_cannot_share_a_memory_id() {
    const FIRST_IDS: &[MemoryId] = &[MemoryId::new(70)];
    const SECOND_IDS: &[MemoryId] = &[MemoryId::new(70)];
    let definitions = [
        AllocationDefinition {
            key: StateAllocationKey::BlobStorageRoots,
            owner: AllocationOwner::CanicCore,
            memory_ids: FIRST_IDS,
        },
        AllocationDefinition {
            key: StateAllocationKey::BlobStoragePendingDeletions,
            owner: AllocationOwner::CanicCore,
            memory_ids: SECOND_IDS,
        },
    ];

    assert_eq!(
        allocation::validate_allocation_definitions(&definitions),
        Err(RoleContractFinding::MemoryIdCollision {
            memory_id: MemoryId::new(70),
            first: StateAllocationKey::BlobStorageRoots,
            second: StateAllocationKey::BlobStoragePendingDeletions,
        })
    );
}

#[test]
fn allocation_owners_cannot_claim_another_owner_range() {
    const CONTROL_PLANE_ID: &[MemoryId] = &[MemoryId::new(allocation::CANIC_CONTROL_PLANE_MIN_ID)];
    const CORE_ID: &[MemoryId] = &[MemoryId::new(allocation::CANIC_CORE_MAX_ID)];

    for definition in [
        AllocationDefinition {
            key: StateAllocationKey::BlobStorageRoots,
            owner: AllocationOwner::CanicCore,
            memory_ids: CONTROL_PLANE_ID,
        },
        AllocationDefinition {
            key: StateAllocationKey::TemplateManifests,
            owner: AllocationOwner::CanicControlPlane,
            memory_ids: CORE_ID,
        },
    ] {
        assert!(matches!(
            allocation::validate_allocation_definitions(&[definition]),
            Err(RoleContractFinding::CatalogInvalid { .. })
        ));
    }
}

#[test]
fn capability_derivation_is_centralized_for_auth_and_sharding() {
    let mut app = ConfigTestBuilder::canister_config(CanisterKind::Service);
    app.auth = CanisterAuthConfig {
        delegated_token_issuer: false,
        delegated_token_verifier: true,
        local_application_authorization: None,
        role_attestation_cache: true,
    };
    app.sharding = Some(ShardingConfig::default());
    let config = ConfigTestBuilder::new()
        .with_default_canister("app", app)
        .with_fleet_admission("app")
        .build();
    let role = CanisterRole::owned("app".to_string());

    let first = derive_role_capabilities(&config, &role).expect("known role should resolve");
    let second = derive_role_capabilities(&config, &role).expect("known role should resolve");
    assert_eq!(first, second);
    assert_eq!(
        first,
        BTreeSet::from([
            RoleCapabilityKey::DelegatedTokenVerifier,
            RoleCapabilityKey::FleetAdmissionProjection,
            RoleCapabilityKey::RoleAttestationVerifier,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::Sharding,
        ])
    );
}

#[test]
fn child_provisioning_is_derived_only_for_roles_with_spawn_grants() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind("project_instance", CanisterKind::Service)
        .with_default_canister_kind("project_machine", CanisterKind::Instance)
        .build();

    let parent = derive_role_capabilities(&config, &CanisterRole::new("project_instance"))
        .expect("parent role should resolve");
    let child = derive_role_capabilities(&config, &CanisterRole::new("project_machine"))
        .expect("child role should resolve");

    assert!(parent.contains(&RoleCapabilityKey::ChildProvisioning));
    assert!(!child.contains(&RoleCapabilityKey::ChildProvisioning));
}

#[test]
fn local_application_authorization_capability_is_exactly_role_pruned() {
    let mut enabled = ConfigTestBuilder::canister_config(CanisterKind::Service);
    enabled.auth.delegated_token_verifier = true;
    enabled.auth.local_application_authorization = Some(LocalApplicationAuthorizationConfig {
        allowed_scopes: vec!["app:read".to_string()],
        default_session_ttl_secs: 900,
        maximum_session_ttl_secs: 1_800,
    });
    let disabled = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
    let config = ConfigTestBuilder::new()
        .with_default_canister("enabled", enabled)
        .with_default_canister("disabled", disabled)
        .with_fleet_admission("enabled")
        .build();

    let enabled = derive_role_capabilities(&config, &CanisterRole::new("enabled")).unwrap();
    let disabled = derive_role_capabilities(&config, &CanisterRole::new("disabled")).unwrap();
    assert!(enabled.contains(&RoleCapabilityKey::LocalApplicationAuthorization));
    assert!(enabled.contains(&RoleCapabilityKey::DelegatedTokenVerifier));
    assert!(enabled.contains(&RoleCapabilityKey::FleetAdmissionProjection));
    assert!(!disabled.contains(&RoleCapabilityKey::FleetAdmissionProjection));
    assert!(!disabled.contains(&RoleCapabilityKey::LocalApplicationAuthorization));
    assert!(
        !built_in_role_capabilities(BuiltInRoleKind::FleetCoordinator)
            .contains(&RoleCapabilityKey::LocalApplicationAuthorization)
    );
    assert!(
        !built_in_role_capabilities(BuiltInRoleKind::WasmStore)
            .contains(&RoleCapabilityKey::LocalApplicationAuthorization)
    );
}

#[test]
fn fleet_admission_projection_allocation_requires_explicit_nonroot_enrollment() {
    let role = CanisterRole::new("service");
    let config = ConfigTestBuilder::new()
        .with_default_canister(
            role.clone(),
            ConfigTestBuilder::canister_config(CanisterKind::Service),
        )
        .with_fleet_admission(role.clone())
        .build();
    let RoleContractResolution::Resolved { contract: service } =
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: BTreeSet::new(),
            default_features_enabled: true,
        })
    else {
        panic!("enrolled service role contract should resolve");
    };
    assert!(
        service
            .allocations
            .iter()
            .any(|allocation| allocation.key == StateAllocationKey::CoreFleetAdmissionProjection)
    );
    assert!(
        service
            .capabilities
            .contains(&RoleCapabilityKey::FleetAdmissionProjection)
    );

    let root_config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .build();
    let RoleContractResolution::Resolved { contract: root } =
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &root_config,
                role: &CanisterRole::ROOT,
            },
            declared_features: BTreeSet::from([CanicFeatureKey::ControlPlane]),
            default_features_enabled: true,
        })
    else {
        panic!("root contract should resolve");
    };
    assert!(
        !root
            .allocations
            .iter()
            .any(|allocation| allocation.key == StateAllocationKey::CoreFleetAdmissionProjection)
    );

    for (role, declared_feature) in [
        (
            BuiltInRoleKind::FleetCoordinator,
            CanicFeatureKey::FleetCoordinatorCanister,
        ),
        (
            BuiltInRoleKind::WasmStore,
            CanicFeatureKey::WasmStoreCanister,
        ),
    ] {
        let RoleContractResolution::Resolved { contract } =
            resolve_role_contract(RoleContractInput {
                source: RoleContractSource::BuiltIn(role),
                declared_features: BTreeSet::from([declared_feature]),
                default_features_enabled: false,
            })
        else {
            panic!("built-in contract should resolve");
        };
        assert!(
            !contract.allocations.iter().any(
                |allocation| allocation.key == StateAllocationKey::CoreFleetAdmissionProjection
            )
        );
    }
}

#[test]
fn automatic_topup_is_derived_only_from_the_exact_configured_role() {
    let mut funded = ConfigTestBuilder::canister_config(CanisterKind::Service);
    funded.topup = Some(TopupPolicy::default());
    let plain = ConfigTestBuilder::canister_config(CanisterKind::Singleton);
    let config = ConfigTestBuilder::new()
        .with_default_canister("funded", funded)
        .with_default_canister("plain", plain)
        .build();

    let funded = derive_role_capabilities(&config, &CanisterRole::owned("funded".to_string()))
        .expect("funded role should resolve");
    let plain = derive_role_capabilities(&config, &CanisterRole::owned("plain".to_string()))
        .expect("plain role should resolve");

    assert!(funded.contains(&RoleCapabilityKey::AutomaticTopup));
    assert!(!plain.contains(&RoleCapabilityKey::AutomaticTopup));
    assert_eq!(
        built_in_role_capabilities(BuiltInRoleKind::WasmStore),
        BTreeSet::from([
            RoleCapabilityKey::ChildProvisioning,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::WasmStore,
        ])
    );
}

#[test]
fn root_inherently_selects_icp_refill_state() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .build();

    let RoleContractResolution::Resolved { contract } = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &CanisterRole::ROOT,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::ControlPlane]),
        default_features_enabled: true,
    }) else {
        panic!("root contract should resolve");
    };

    let allocation = contract
        .allocations
        .iter()
        .find(|allocation| allocation.key == StateAllocationKey::CoreCyclesIcpRefillRecords)
        .expect("ICP refill state allocation");
    assert_eq!(allocation.memory_ids, vec![MemoryId::new(39)]);
    assert_eq!(
        allocation.selected_by,
        BTreeSet::from([SelectionProvenance::Capability(RoleCapabilityKey::Root)])
    );
}

#[test]
fn placement_capabilities_select_only_their_placement_state() {
    let mut scaling = ConfigTestBuilder::canister_config(CanisterKind::Service);
    scaling.scaling = Some(ScalingConfig::default());
    assert_eq!(
        placement_allocation_ids(&resolved_service_contract(scaling, BTreeSet::new()).allocations),
        vec![50]
    );

    let mut index = ConfigTestBuilder::canister_config(CanisterKind::Service);
    index.index = Some(IndexConfig::default());
    assert_eq!(
        placement_allocation_ids(&resolved_service_contract(index, BTreeSet::new()).allocations),
        vec![51]
    );

    let mut sharding = ConfigTestBuilder::canister_config(CanisterKind::Service);
    sharding.sharding = Some(ShardingConfig::default());
    let contract = resolved_service_contract(sharding, BTreeSet::from([CanicFeatureKey::Sharding]));
    assert_eq!(
        placement_allocation_ids(&contract.allocations),
        vec![52, 53, 54]
    );
}

#[test]
fn feature_implication_closure_is_idempotent() {
    let direct = BTreeSet::from([
        CanicFeatureKey::AuthDelegatedTokenVerify,
        CanicFeatureKey::BlobStorageBilling,
    ]);
    let first = resolve_effective_features(direct, true);
    let second = resolve_effective_features(first.clone(), false);

    assert_eq!(first, second);
    assert!(first.contains(&CanicFeatureKey::AuthChainKeyEcdsa));
    assert!(first.contains(&CanicFeatureKey::AuthIssuerCanisterSigVerify));
    assert!(first.contains(&CanicFeatureKey::BlobStorage));
}

#[test]
fn missing_required_feature_rejects_without_a_contract() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .build();
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &CanisterRole::ROOT,
        },
        declared_features: BTreeSet::new(),
        default_features_enabled: true,
    });

    assert_eq!(
        resolution,
        RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::RequiredFeatureMissing {
                capability: RoleCapabilityKey::RootControlPlane,
                feature: CanicFeatureKey::ControlPlane,
            }],
        }
    );
}

#[test]
fn unknown_role_rejects_without_a_contract() {
    let config = ConfigTestBuilder::new().build();
    let role = CanisterRole::owned("missing".to_string());

    assert_eq!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: CanicFeatureKey::ALL.iter().copied().collect(),
            default_features_enabled: true,
        }),
        RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::RoleUnknown { role }],
        }
    );
}

#[test]
fn surplus_state_feature_allocates_normally() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind("app", CanisterKind::Service)
        .build();
    let role = CanisterRole::owned("app".to_string());
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &role,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::BlobStorageBilling]),
        default_features_enabled: true,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("surplus state-bearing features should resolve normally");
    };

    assert_eq!(
        allocation_ids(&contract.allocations),
        vec![
            30, 31, 32, 33, 35, 36, 37, 38, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 55, 56, 57, 58,
            60,
        ]
    );
}

#[test]
fn repeated_selection_merges_allocation_provenance() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .build();
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &CanisterRole::ROOT,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::ControlPlane]),
        default_features_enabled: true,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("root contract should resolve");
    };
    let template_manifests = contract
        .allocations
        .iter()
        .find(|allocation| allocation.key == StateAllocationKey::TemplateManifests)
        .expect("root should own template manifests");

    assert_eq!(
        template_manifests.selected_by,
        BTreeSet::from([
            SelectionProvenance::Capability(RoleCapabilityKey::RootControlPlane),
            SelectionProvenance::EffectiveFeature(CanicFeatureKey::ControlPlane),
        ])
    );
    assert_eq!(
        allocation_ids(&contract.allocations),
        vec![
            10, 11, 12, 13, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33,
            34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 59, 60, 63, 65,
        ]
    );
}

#[test]
fn built_in_wasm_store_keeps_template_and_gc_ids() {
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::BuiltIn(BuiltInRoleKind::WasmStore),
        declared_features: BTreeSet::from([CanicFeatureKey::WasmStoreCanister]),
        default_features_enabled: false,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("built-in wasm_store contract should resolve");
    };

    assert_eq!(
        allocation_ids(&contract.allocations),
        vec![
            10, 11, 12, 13, 14, 30, 31, 32, 33, 35, 36, 37, 38, 40, 41, 42, 43, 44, 45, 46, 47, 48,
            49, 60,
        ]
    );
    assert_eq!(
        contract.required_features,
        BTreeSet::from([CanicFeatureKey::WasmStoreCanister])
    );
}

#[test]
fn built_in_fleet_coordinator_selects_admission_registry_funding_and_restore_fence() {
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::BuiltIn(BuiltInRoleKind::FleetCoordinator),
        declared_features: BTreeSet::from([CanicFeatureKey::FleetCoordinatorCanister]),
        default_features_enabled: false,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("built-in Fleet Coordinator contract should resolve");
    };

    assert_eq!(allocation_ids(&contract.allocations), vec![15, 59, 62, 64]);
    assert_eq!(
        contract.required_features,
        BTreeSet::from([CanicFeatureKey::FleetCoordinatorCanister])
    );
}

fn allocation_ids(allocations: &[super::ResolvedStateAllocation]) -> Vec<u8> {
    let mut ids = allocations
        .iter()
        .flat_map(|allocation| allocation.memory_ids.iter())
        .map(|memory_id| memory_id.get())
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn placement_allocation_ids(allocations: &[super::ResolvedStateAllocation]) -> Vec<u8> {
    allocation_ids(allocations)
        .into_iter()
        .filter(|memory_id| (50..=54).contains(memory_id))
        .collect()
}

fn resolved_service_contract(
    canister: CanisterConfig,
    declared_features: BTreeSet<CanicFeatureKey>,
) -> super::ResolvedRoleContract {
    let role = CanisterRole::owned("service".to_string());
    let config = ConfigTestBuilder::new()
        .with_default_canister(role.clone(), canister)
        .build();
    let RoleContractResolution::Resolved { contract } = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &role,
        },
        declared_features,
        default_features_enabled: true,
    }) else {
        panic!("service role contract should resolve");
    };
    contract
}

fn read_manifest(relative_path: &str) -> toml::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative_path);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    toml::from_str(&source)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

fn feature_table(manifest: &toml::Value) -> &toml::map::Map<String, toml::Value> {
    manifest
        .get("features")
        .and_then(toml::Value::as_table)
        .expect("manifest should have a feature table")
}

fn feature_members(features: &toml::map::Map<String, toml::Value>, feature: &str) -> Vec<String> {
    features
        .get(feature)
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("feature {feature} should be an array"))
        .iter()
        .map(|member| {
            member
                .as_str()
                .unwrap_or_else(|| panic!("feature {feature} should contain strings"))
                .to_string()
        })
        .collect()
}

fn cargo_public_implications(
    canic_features: &toml::map::Map<String, toml::Value>,
    core_features: &toml::map::Map<String, toml::Value>,
    public_features: &BTreeSet<String>,
) -> BTreeSet<(String, String)> {
    let mut implications = BTreeSet::new();

    for feature in public_features {
        for member in feature_members(canic_features, feature) {
            if public_features.contains(&member) {
                implications.insert((feature.clone(), member));
                continue;
            }

            let Some(core_feature) = member.strip_prefix("canic-core/") else {
                continue;
            };
            for core_member in feature_members(core_features, core_feature) {
                if public_features.contains(&core_member) {
                    implications.insert((feature.clone(), core_member));
                }
            }
        }
    }

    implications
}
