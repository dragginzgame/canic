use super::{
    AllocationDefinition, AllocationOwner, BuiltInRoleKind, CanicFeatureEffect, CanicFeatureKey,
    MemoryId, RoleCapabilityKey, RoleContractFinding, RoleContractInput, RoleContractResolution,
    RoleContractSource, SelectionProvenance, StateAllocationKey, allocation,
    catalog::{self, default_features, implied_features},
    derive_role_capabilities, resolve_effective_features, resolve_role_contract,
};
use crate::{
    cdk::types::Cycles,
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, DirectoryConfig, IcpRefillPolicy,
        ScalingConfig, ShardingConfig, TopupPolicy,
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
        .filter(|name| name.as_str() != "default")
        .cloned()
        .collect::<BTreeSet<_>>();
    let catalog_public_features = catalog::feature_definitions()
        .iter()
        .map(|definition| definition.cargo_name.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(catalog_public_features, cargo_public_features);

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
        (
            StateAllocationKey::CoreRuntimeTopology,
            vec![11, 12, 13, 15],
        ),
        (StateAllocationKey::CoreRootAppRegistry, vec![14]),
        (StateAllocationKey::CoreRuntimeEnvironment, vec![16, 17, 18]),
        (StateAllocationKey::CoreAuthState, vec![19]),
        (StateAllocationKey::CoreReplayReceipts, vec![20]),
        (
            StateAllocationKey::CoreRuntimeObservability,
            vec![29, 30, 31, 32, 34],
        ),
        (StateAllocationKey::CoreIcpRefillRecords, vec![33]),
        (
            StateAllocationKey::CoreRuntimeIntent,
            vec![39, 40, 41, 42, 43, 44],
        ),
        (StateAllocationKey::CanisterPool, vec![49]),
        (StateAllocationKey::ScalingRegistry, vec![52]),
        (StateAllocationKey::DirectoryRegistry, vec![55]),
        (StateAllocationKey::ShardingRegistry, vec![53]),
        (StateAllocationKey::ShardingAssignments, vec![54]),
        (StateAllocationKey::ShardingActiveSet, vec![56]),
        (StateAllocationKey::StoredBlobs, vec![62]),
        (StateAllocationKey::BlobDeletionPending, vec![63]),
        (StateAllocationKey::StorageGatewayPrincipals, vec![64]),
        (StateAllocationKey::BlobStorageBilling, vec![65]),
        (StateAllocationKey::TemplateManifests, vec![80]),
        (StateAllocationKey::TemplateChunkSets, vec![81]),
        (StateAllocationKey::TemplateChunkRefs, vec![82]),
        (StateAllocationKey::TemplateChunkPayloads, vec![83]),
        (StateAllocationKey::ControlPlaneSubnetState, vec![84]),
        (StateAllocationKey::WasmStoreGcState, vec![85]),
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn distinct_allocation_keys_cannot_share_a_memory_id() {
    const FIRST_IDS: &[MemoryId] = &[MemoryId::new(70)];
    const SECOND_IDS: &[MemoryId] = &[MemoryId::new(70)];
    let definitions = [
        AllocationDefinition {
            key: StateAllocationKey::StoredBlobs,
            owner: AllocationOwner::CanicCore,
            memory_ids: FIRST_IDS,
        },
        AllocationDefinition {
            key: StateAllocationKey::BlobDeletionPending,
            owner: AllocationOwner::CanicCore,
            memory_ids: SECOND_IDS,
        },
    ];

    assert_eq!(
        allocation::validate_allocation_definitions(&definitions),
        Err(RoleContractFinding::MemoryIdCollision {
            memory_id: MemoryId::new(70),
            first: StateAllocationKey::StoredBlobs,
            second: StateAllocationKey::BlobDeletionPending,
        })
    );
}

#[test]
fn allocation_owners_cannot_claim_another_owner_range() {
    const CONTROL_PLANE_ID: &[MemoryId] = &[MemoryId::new(allocation::CANIC_CONTROL_PLANE_MIN_ID)];
    const CORE_ID: &[MemoryId] = &[MemoryId::new(allocation::CANIC_CORE_MAX_ID)];

    for definition in [
        AllocationDefinition {
            key: StateAllocationKey::StoredBlobs,
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
        role_attestation_cache: true,
    };
    app.sharding = Some(ShardingConfig::default());
    let config = ConfigTestBuilder::new()
        .with_prime_canister("app", app)
        .build();
    let role = CanisterRole::owned("app".to_string());

    let first = derive_role_capabilities(&config, &role).expect("known role should resolve");
    let second = derive_role_capabilities(&config, &role).expect("known role should resolve");
    assert_eq!(first, second);
    assert_eq!(
        first,
        BTreeSet::from([
            RoleCapabilityKey::DelegatedTokenVerifier,
            RoleCapabilityKey::RoleAttestationVerifier,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::Sharding,
        ])
    );
}

#[test]
fn icp_refill_config_requires_its_feature_and_selects_its_state() {
    let mut app = ConfigTestBuilder::canister_config(CanisterKind::Service);
    app.topup = Some(TopupPolicy {
        icp_refill: Some(IcpRefillPolicy {
            enabled: true,
            min_hub_cycles_before_refill: Cycles::from(2_000_000_000_000_u128),
            max_refill_e8s_per_call: 100_000_000,
            min_xdr_permyriad_per_icp: Some(40_000),
            ledger_canister_id: None,
            cmc_canister_id: None,
            allow_ic_system_canister_overrides: false,
        }),
        ..TopupPolicy::default()
    });
    let config = ConfigTestBuilder::new()
        .with_prime_canister("app", app)
        .build();
    let role = CanisterRole::owned("app".to_string());

    let RoleContractResolution::Resolved { contract } = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &role,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::IcpRefill]),
        default_features_enabled: true,
    }) else {
        panic!("ICP refill contract should resolve");
    };

    assert!(
        contract
            .capabilities
            .contains(&RoleCapabilityKey::IcpRefill)
    );
    assert!(
        contract
            .required_features
            .contains(&CanicFeatureKey::IcpRefill)
    );
    let allocation = contract
        .allocations
        .iter()
        .find(|allocation| allocation.key == StateAllocationKey::CoreIcpRefillRecords)
        .expect("ICP refill state allocation");
    assert_eq!(allocation.memory_ids, vec![MemoryId::new(33)]);
    assert_eq!(
        allocation.selected_by,
        BTreeSet::from([
            SelectionProvenance::Capability(RoleCapabilityKey::IcpRefill),
            SelectionProvenance::EffectiveFeature(CanicFeatureKey::IcpRefill),
        ])
    );
}

#[test]
fn placement_capabilities_select_only_their_placement_state() {
    let mut scaling = ConfigTestBuilder::canister_config(CanisterKind::Service);
    scaling.scaling = Some(ScalingConfig::default());
    assert_eq!(
        placement_allocation_ids(&resolved_service_contract(scaling, BTreeSet::new()).allocations),
        vec![49, 52]
    );

    let mut directory = ConfigTestBuilder::canister_config(CanisterKind::Service);
    directory.directory = Some(DirectoryConfig::default());
    assert_eq!(
        placement_allocation_ids(
            &resolved_service_contract(directory, BTreeSet::new()).allocations
        ),
        vec![49, 55]
    );

    let mut sharding = ConfigTestBuilder::canister_config(CanisterKind::Service);
    sharding.sharding = Some(ShardingConfig::default());
    let contract = resolved_service_contract(sharding, BTreeSet::from([CanicFeatureKey::Sharding]));
    assert_eq!(
        placement_allocation_ids(&contract.allocations),
        vec![49, 53, 54, 56]
    );
    let pool = contract
        .allocations
        .iter()
        .find(|allocation| allocation.key == StateAllocationKey::CanisterPool)
        .expect("sharding selects the shared canister pool");
    assert_eq!(
        pool.selected_by,
        BTreeSet::from([
            SelectionProvenance::Capability(RoleCapabilityKey::Sharding),
            SelectionProvenance::EffectiveFeature(CanicFeatureKey::Sharding),
        ])
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
    assert!(first.contains(&CanicFeatureKey::Metrics));
}

#[test]
fn missing_required_feature_rejects_without_a_contract() {
    let config = ConfigTestBuilder::new()
        .with_prime_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
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
        .with_prime_canister_kind("app", CanisterKind::Service)
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
            11, 12, 13, 15, 16, 17, 18, 20, 29, 30, 31, 32, 34, 39, 40, 41, 42, 43, 44, 62, 63, 64,
            65,
        ]
    );
}

#[test]
fn repeated_selection_merges_allocation_provenance() {
    let config = ConfigTestBuilder::new()
        .with_prime_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
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
            11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 29, 30, 31, 32, 34, 39, 40, 41, 42, 43, 44, 49,
            80, 81, 82, 83, 84,
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
            11, 12, 13, 15, 16, 17, 18, 20, 29, 30, 31, 32, 34, 39, 40, 41, 42, 43, 44, 80, 81, 82,
            83, 85,
        ]
    );
    assert_eq!(
        contract.required_features,
        BTreeSet::from([CanicFeatureKey::WasmStoreCanister])
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
        .filter(|memory_id| (49..=56).contains(memory_id))
        .collect()
}

fn resolved_service_contract(
    canister: CanisterConfig,
    declared_features: BTreeSet<CanicFeatureKey>,
) -> super::ResolvedRoleContract {
    let role = CanisterRole::owned("service".to_string());
    let config = ConfigTestBuilder::new()
        .with_prime_canister(role.clone(), canister)
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
