use super::{
    AllocationDefinition, AllocationOwner, BuiltInRoleKind, CanicFeatureEffect, CanicFeatureKey,
    RoleCapabilityKey, RoleContractFinding, RoleContractInput, RoleContractResolution,
    RoleContractSource, SelectionProvenance, StateAllocationKey, allocation,
    built_in_role_capabilities, catalog,
    catalog::{default_features, implied_features},
    derive_role_capabilities, required_features_for_role, resolve_effective_features,
    resolve_role_contract,
};
use crate::{
    config::schema::{
        CanisterAuthConfig, CanisterConfig, CanisterKind, IndexConfig,
        LocalApplicationAuthorizationConfig, ScalingConfig, ShardingConfig, TopupPolicy,
    },
    test::config::ConfigTestBuilder,
};
use canic_contracts::ids::CanisterRole;
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
fn optional_observability_changes_capabilities_without_changing_state_ownership() {
    let mut config = ConfigTestBuilder::new()
        .with_default_canister(
            "app",
            ConfigTestBuilder::canister_config(CanisterKind::Service),
        )
        .build();
    let role = CanisterRole::new("app");
    let resolve = |config: &crate::config::schema::ConfigModel| match resolve_role_contract(
        RoleContractInput {
            source: RoleContractSource::Declared {
                config,
                role: &role,
            },
            declared_features: BTreeSet::new(),
            default_features_enabled: false,
        },
    ) {
        RoleContractResolution::Resolved { contract } => contract,
        other @ RoleContractResolution::Rejected { .. } => {
            panic!("unexpected resolution: {other:?}")
        }
    };
    let full = resolve(&config);
    let selection: crate::config::schema::RoleObservabilityConfig =
        toml::from_str("diagnostics = false\nhistory = false\nlogs = false\nmetrics = false\n")
            .unwrap();
    config.roles.get_mut(&role).unwrap().observability = selection;
    let lean = resolve(&config);
    assert_eq!(full.allocations, lean.allocations);
    assert_eq!(
        full.capabilities
            .difference(&lean.capabilities)
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            RoleCapabilityKey::ObservabilityDiagnostics,
            RoleCapabilityKey::ObservabilityHistory,
            RoleCapabilityKey::ObservabilityLogs,
            RoleCapabilityKey::ObservabilityMetrics,
        ])
    );
    assert!(lean.capabilities.contains(&RoleCapabilityKey::Runtime));
    assert!(
        toml::from_str::<crate::config::schema::RoleObservabilityConfig>("metric = false").is_err()
    );
    let partial: crate::config::schema::RoleObservabilityConfig =
        toml::from_str("logs = false").unwrap();
    assert!(partial.metrics && partial.history && partial.diagnostics && !partial.logs);
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
#[expect(
    clippy::too_many_lines,
    reason = "the independent permanent-key fixture covers every canonical allocation group"
)]
fn canonical_allocations_match_the_active_memory_map() {
    allocation::validate_canonical_allocations()
        .expect("canonical allocation definitions should be valid");

    let actual = allocation::allocation_definitions()
        .iter()
        .map(|definition| {
            (
                definition.key,
                definition
                    .memory_keys
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected = BTreeMap::from([
        (
            StateAllocationKey::CoreRuntimeChildren,
            vec!["canic.core.runtime.canister_children.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreRuntimeBindings,
            vec!["canic.core.runtime.bindings.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreFleetState,
            vec!["canic.core.fleet.state.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreFleetActivation,
            vec!["canic.core.fleet.activation.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreCallerAuthority,
            vec![
                "canic.core.caller_authority.header.v1".to_string(),
                "canic.core.caller_authority.rows.v1".to_string(),
            ],
        ),
        (
            StateAllocationKey::CoreLocalApplicationAuthorizationState,
            vec!["canic.core.auth.local_application_authorization.state.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreReplayReceipts,
            vec!["canic.core.replay.receipts.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreCycles,
            vec![
                "canic.core.cycles.tracker.v1".to_string(),
                "canic.core.cycles.topup_events.v1".to_string(),
                "canic.core.cycles.funding_ledger.v1".to_string(),
            ],
        ),
        (
            StateAllocationKey::CoreCyclesIcpRefillRecords,
            vec!["canic.core.cycles.icp_refill_records.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreRuntimeLog,
            vec!["canic.core.log.entries.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreIntent,
            vec![
                "canic.core.intent.meta.v1".to_string(),
                "canic.core.intent.records.v1".to_string(),
                "canic.core.intent.totals.v1".to_string(),
                "canic.core.intent.pending.v1".to_string(),
                "canic.core.intent.receipt_backed_records.v1".to_string(),
                "canic.core.intent.expiry_index.v1".to_string(),
            ],
        ),
        (
            StateAllocationKey::CoreApplicationReceipts,
            vec!["canic.core.application_receipt.eligibility.v1".to_string()],
        ),
        (
            StateAllocationKey::CorePlacementAcknowledgement,
            vec!["canic.core.placement.acknowledgement_index.v1".to_string()],
        ),
        (
            StateAllocationKey::PlacementScalingRegistry,
            vec!["canic.core.placement.scaling_registry.v1".to_string()],
        ),
        (
            StateAllocationKey::PlacementIndexRegistry,
            vec!["canic.core.placement.index_registry.v1".to_string()],
        ),
        (
            StateAllocationKey::ShardingRegistry,
            vec!["canic.core.sharding.registry.v1".to_string()],
        ),
        (
            StateAllocationKey::ShardingAssignments,
            vec!["canic.core.sharding.assignments.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreAuthorityRestoreFence,
            vec!["canic.core.authority_restore.fence.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreAsyncJobRecovery,
            vec!["canic.core.async_job_recovery.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreFleetAdmissionProjection,
            vec!["canic.core.fleet_admission.projection.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreDelegatedTokenIssuerState,
            vec!["canic.core.auth.delegated_token_issuer.state.v1".to_string()],
        ),
        (
            StateAllocationKey::CoreRootDelegationState,
            vec!["canic.core.auth.root_delegation.state.v1".to_string()],
        ),
        (
            StateAllocationKey::FleetCoordinatorFunding,
            vec!["canic.control_plane.fleet_coordinator.funding.v1".to_string()],
        ),
        (
            StateAllocationKey::RootFunding,
            vec!["canic.control_plane.root.funding.v1".to_string()],
        ),
        (
            StateAllocationKey::FleetCoordinatorAdmission,
            vec!["canic.control_plane.fleet_admission.v1".to_string()],
        ),
        (
            StateAllocationKey::RootAdmission,
            vec!["canic.control_plane.root.admission.v1".to_string()],
        ),
        (
            StateAllocationKey::TemplateManifests,
            vec!["canic.control_plane.template.manifests.v1".to_string()],
        ),
        (
            StateAllocationKey::TemplateChunkSets,
            vec!["canic.control_plane.template.chunk_sets.v1".to_string()],
        ),
        (
            StateAllocationKey::TemplateChunkRefs,
            vec!["canic.control_plane.template.chunk_refs.v1".to_string()],
        ),
        (
            StateAllocationKey::TemplateChunkPayloads,
            vec!["canic.control_plane.template.chunk_payloads.v1".to_string()],
        ),
        (
            StateAllocationKey::WasmStoreGcState,
            vec!["canic.control_plane.wasm_store.gc_state.v1".to_string()],
        ),
        (
            StateAllocationKey::FixtureStore,
            vec!["canic.control_plane.fixture_store.v1".to_string()],
        ),
        (
            StateAllocationKey::FleetCoordinatorRegistry,
            vec!["canic.control_plane.fleet_coordinator.registry.v1".to_string()],
        ),
        (
            StateAllocationKey::RootWasmStoreState,
            vec!["canic.control_plane.root.wasm_store.state.v1".to_string()],
        ),
        (
            StateAllocationKey::RootFleetRegistryMirror,
            vec!["canic.control_plane.root.fleet_registry_mirror.v1".to_string()],
        ),
        (
            StateAllocationKey::RootComponentRegistry,
            vec![
                "canic.control_plane.root.component.registry_state.v1".to_string(),
                "canic.control_plane.root.component.allocations.v1".to_string(),
                "canic.control_plane.root.component.registry_entries.v1".to_string(),
                "canic.control_plane.root.component.principal_index.v1".to_string(),
                "canic.control_plane.root.component.subtree_removal_history.v1".to_string(),
                "canic.control_plane.root.component.draining.v1".to_string(),
            ],
        ),
        (
            StateAllocationKey::RootCanisterPool,
            vec![
                "canic.control_plane.root.canister_inventory.assets.v1".to_string(),
                "canic.control_plane.root.canister_pool.state.v1".to_string(),
                "canic.control_plane.root.canister_pool.handoff_receipts.v1".to_string(),
            ],
        ),
        (
            StateAllocationKey::RootComponentProvisioning,
            vec![
                "canic.control_plane.root.component_provisioning.operations.v1".to_string(),
                "canic.control_plane.root.component_provisioning.placements.v1".to_string(),
                "canic.control_plane.root.component_provisioning.state.v1".to_string(),
            ],
        ),
    ]);
    assert_eq!(actual, expected);
}

#[test]
fn distinct_allocation_groups_cannot_share_a_memory_key() {
    const FIRST_IDS: &[&str] = &["canic.core.shared.v1"];
    const SECOND_IDS: &[&str] = &["canic.core.shared.v1"];
    let definitions = [
        AllocationDefinition {
            key: StateAllocationKey::ShardingRegistry,
            owner: AllocationOwner::CanicCore,
            memory_keys: FIRST_IDS,
        },
        AllocationDefinition {
            key: StateAllocationKey::ShardingAssignments,
            owner: AllocationOwner::CanicCore,
            memory_keys: SECOND_IDS,
        },
    ];

    assert_eq!(
        allocation::validate_allocation_definitions(&definitions),
        Err(RoleContractFinding::MemoryKeyCollision {
            stable_key: "canic.core.shared.v1".to_string(),
            first: StateAllocationKey::ShardingRegistry,
            second: StateAllocationKey::ShardingAssignments,
        })
    );
}

#[test]
fn allocation_owners_cannot_claim_another_owner_namespace() {
    const CONTROL_PLANE_ID: &[&str] = &["canic.control_plane.rows.v1"];
    const CORE_ID: &[&str] = &["canic.core.rows.v1"];

    for definition in [
        AllocationDefinition {
            key: StateAllocationKey::ShardingRegistry,
            owner: AllocationOwner::CanicCore,
            memory_keys: CONTROL_PLANE_ID,
        },
        AllocationDefinition {
            key: StateAllocationKey::TemplateManifests,
            owner: AllocationOwner::CanicControlPlane,
            memory_keys: CORE_ID,
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
            RoleCapabilityKey::CallerAuthority,
            RoleCapabilityKey::DelegatedTokenVerifier,
            RoleCapabilityKey::FleetAdmissionProjection,
            RoleCapabilityKey::ObservabilityDiagnostics,
            RoleCapabilityKey::ObservabilityHistory,
            RoleCapabilityKey::ObservabilityLogs,
            RoleCapabilityKey::ObservabilityMetrics,
            RoleCapabilityKey::RoleAttestationVerifier,
            RoleCapabilityKey::Runtime,
            RoleCapabilityKey::Sharding,
        ])
    );
}

#[test]
fn role_attestation_cache_requires_chain_key_validation() {
    let mut app = ConfigTestBuilder::canister_config(CanisterKind::Service);
    app.auth.role_attestation_cache = true;
    let config = ConfigTestBuilder::new()
        .with_default_canister("app", app)
        .build();
    let role = CanisterRole::new("app");

    assert_eq!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: BTreeSet::from([CanicFeatureKey::AuthRootCanisterSigVerify]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::RequiredFeatureMissing {
                capability: RoleCapabilityKey::RoleAttestationVerifier,
                feature: CanicFeatureKey::AuthChainKeyEcdsa,
            }],
        }
    );

    assert!(matches!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: BTreeSet::from([
                CanicFeatureKey::AuthRootCanisterSigVerify,
                CanicFeatureKey::AuthChainKeyEcdsa,
            ]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Resolved { .. }
    ));
}

#[test]
fn root_chain_key_signing_feature_is_required_only_when_an_issuer_exists() {
    let mut issuer = ConfigTestBuilder::canister_config(CanisterKind::Shard);
    issuer.auth.delegated_token_issuer = true;
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .with_default_canister("issuer", issuer)
        .build();

    let requirements = required_features_for_role(&config, &CanisterRole::ROOT)
        .expect("configured Root role should resolve");
    assert!(requirements.iter().any(|requirement| {
        requirement.capability == RoleCapabilityKey::RootDelegation
            && requirement.feature == CanicFeatureKey::AuthChainKeyRootSign
    }));

    assert!(matches!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &CanisterRole::ROOT,
            },
            declared_features: BTreeSet::from([CanicFeatureKey::ControlPlane]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Rejected { errors }
            if errors == vec![RoleContractFinding::RequiredFeatureMissing {
                capability: RoleCapabilityKey::RootDelegation,
                feature: CanicFeatureKey::AuthChainKeyRootSign,
            }]
    ));

    assert!(matches!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &CanisterRole::ROOT,
            },
            declared_features: BTreeSet::from([
                CanicFeatureKey::AuthChainKeyRootSign,
                CanicFeatureKey::ControlPlane,
            ]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Resolved { contract } if contract.allocations.iter().any(|allocation| allocation.key == StateAllocationKey::CoreRootDelegationState)
    ));
}

#[test]
fn disabled_delegation_excludes_root_auth_even_with_an_issuer_declaration() {
    let mut issuer = ConfigTestBuilder::canister_config(CanisterKind::Shard);
    issuer.auth.delegated_token_issuer = true;
    let mut config = ConfigTestBuilder::new()
        .with_default_canister_kind(CanisterRole::ROOT, CanisterKind::Root)
        .with_default_canister("issuer", issuer)
        .build();
    config.auth.delegated_tokens.enabled = false;
    let RoleContractResolution::Resolved { contract } = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &CanisterRole::ROOT,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::ControlPlane]),
        default_features_enabled: false,
    }) else {
        panic!("auth-free Root must resolve without cryptographic features");
    };
    assert!(
        !contract
            .capabilities
            .contains(&RoleCapabilityKey::RootDelegation)
    );
    assert!(
        !contract
            .allocations
            .iter()
            .any(|allocation| allocation.key == StateAllocationKey::CoreRootDelegationState)
    );
}

#[test]
fn crypto_features_without_role_authority_are_rejected() {
    let config = ConfigTestBuilder::new()
        .with_default_canister_kind("app", CanisterKind::Service)
        .build();
    let role = CanisterRole::new("app");

    assert_eq!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: BTreeSet::from([CanicFeatureKey::AuthDelegatedTokenVerify]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::SurplusCryptoFeature {
                feature: CanicFeatureKey::AuthDelegatedTokenVerify,
            }],
        }
    );
}

#[test]
fn required_crypto_feature_closure_is_not_treated_as_surplus() {
    let mut app = ConfigTestBuilder::canister_config(CanisterKind::Service);
    app.auth.delegated_token_verifier = true;
    let config = ConfigTestBuilder::new()
        .with_default_canister("app", app)
        .build();
    let role = CanisterRole::new("app");

    let RoleContractResolution::Resolved { contract } = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::Declared {
            config: &config,
            role: &role,
        },
        declared_features: BTreeSet::from([CanicFeatureKey::AuthDelegatedTokenVerify]),
        default_features_enabled: false,
    }) else {
        panic!("exact delegated-token verification feature closure should resolve");
    };
    assert_eq!(
        contract.effective_features,
        BTreeSet::from([
            CanicFeatureKey::AuthChainKeyEcdsa,
            CanicFeatureKey::AuthDelegatedTokenVerify,
            CanicFeatureKey::AuthIssuerCanisterSigVerify,
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
fn auth_capabilities_select_only_their_owned_persistence() {
    let mut verifier = ConfigTestBuilder::canister_config(CanisterKind::Service);
    verifier.auth.delegated_token_verifier = true;

    let mut local = verifier.clone();
    local.auth.local_application_authorization = Some(LocalApplicationAuthorizationConfig {
        allowed_scopes: vec!["app:read".to_string()],
        default_session_ttl_secs: 900,
        maximum_session_ttl_secs: 1_800,
    });

    let mut issuer = ConfigTestBuilder::canister_config(CanisterKind::Service);
    issuer.auth.delegated_token_issuer = true;

    let resolve = |canister: CanisterConfig, declared_features: BTreeSet<CanicFeatureKey>| {
        let role = CanisterRole::new("service");
        let config = ConfigTestBuilder::new()
            .with_default_canister(role.clone(), canister)
            .build();
        let resolution = resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features,
            default_features_enabled: false,
        });
        let RoleContractResolution::Resolved { contract } = resolution else {
            panic!("auth role contract should resolve: {resolution:?}");
        };
        contract
            .allocations
            .into_iter()
            .filter_map(|allocation| match allocation.key {
                StateAllocationKey::CoreDelegatedTokenIssuerState
                | StateAllocationKey::CoreLocalApplicationAuthorizationState
                | StateAllocationKey::CoreRootDelegationState => Some(allocation.key),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
    };

    assert!(
        resolve(
            verifier,
            BTreeSet::from([CanicFeatureKey::AuthDelegatedTokenVerify]),
        )
        .is_empty()
    );
    assert_eq!(
        resolve(
            local,
            BTreeSet::from([CanicFeatureKey::AuthLocalApplicationAuthorization]),
        ),
        BTreeSet::from([StateAllocationKey::CoreLocalApplicationAuthorizationState])
    );
    assert_eq!(
        resolve(
            issuer,
            BTreeSet::from([
                CanicFeatureKey::AuthDelegatedTokenVerify,
                CanicFeatureKey::AuthIssuerCanisterSigCreate,
            ]),
        ),
        BTreeSet::from([StateAllocationKey::CoreDelegatedTokenIssuerState])
    );
}

#[test]
fn local_application_authorization_rejects_the_verifier_only_feature_set() {
    let mut canister = ConfigTestBuilder::canister_config(CanisterKind::Service);
    canister.auth.delegated_token_verifier = true;
    canister.auth.local_application_authorization = Some(LocalApplicationAuthorizationConfig {
        allowed_scopes: vec!["app:read".to_string()],
        default_session_ttl_secs: 900,
        maximum_session_ttl_secs: 1_800,
    });
    let role = CanisterRole::new("service");
    let config = ConfigTestBuilder::new()
        .with_default_canister(role.clone(), canister)
        .build();

    assert_eq!(
        resolve_role_contract(RoleContractInput {
            source: RoleContractSource::Declared {
                config: &config,
                role: &role,
            },
            declared_features: BTreeSet::from([CanicFeatureKey::AuthDelegatedTokenVerify]),
            default_features_enabled: false,
        }),
        RoleContractResolution::Rejected {
            errors: vec![RoleContractFinding::RequiredFeatureMissing {
                capability: RoleCapabilityKey::LocalApplicationAuthorization,
                feature: CanicFeatureKey::AuthLocalApplicationAuthorization,
            }],
        }
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
            RoleCapabilityKey::ObservabilityHistory,
            RoleCapabilityKey::ObservabilityMetrics,
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
    assert_eq!(
        allocation.memory_keys,
        vec!["canic.core.cycles.icp_refill_records.v1".to_string()]
    );
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
        placement_allocation_keys(&resolved_service_contract(scaling, BTreeSet::new()).allocations),
        sorted_keys(vec!["canic.core.placement.scaling_registry.v1".to_string()])
    );

    let mut index = ConfigTestBuilder::canister_config(CanisterKind::Service);
    index.index = Some(IndexConfig::default());
    assert_eq!(
        placement_allocation_keys(&resolved_service_contract(index, BTreeSet::new()).allocations),
        sorted_keys(vec!["canic.core.placement.index_registry.v1".to_string()])
    );

    let mut sharding = ConfigTestBuilder::canister_config(CanisterKind::Service);
    sharding.sharding = Some(ShardingConfig::default());
    let contract = resolved_service_contract(sharding, BTreeSet::from([CanicFeatureKey::Sharding]));
    assert_eq!(
        placement_allocation_keys(&contract.allocations),
        sorted_keys(vec![
            "canic.core.sharding.registry.v1".to_string(),
            "canic.core.sharding.assignments.v1".to_string()
        ])
    );
}

#[test]
fn feature_implication_closure_is_idempotent() {
    let direct = BTreeSet::from([
        CanicFeatureKey::AuthDelegatedTokenVerify,
        CanicFeatureKey::Sharding,
    ]);
    let first = resolve_effective_features(direct, true);
    let second = resolve_effective_features(first.clone(), false);

    assert_eq!(first, second);
    assert!(first.contains(&CanicFeatureKey::AuthChainKeyEcdsa));
    assert!(first.contains(&CanicFeatureKey::AuthIssuerCanisterSigVerify));
    assert!(first.contains(&CanicFeatureKey::Sharding));
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
        declared_features: BTreeSet::from([CanicFeatureKey::Sharding]),
        default_features_enabled: true,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("surplus state-bearing features should resolve normally");
    };

    assert_eq!(
        allocation_keys(&contract.allocations),
        sorted_keys(vec![
            "canic.core.runtime.canister_children.v1".to_string(),
            "canic.core.runtime.bindings.v1".to_string(),
            "canic.core.fleet.state.v1".to_string(),
            "canic.core.fleet.activation.v1".to_string(),
            "canic.core.replay.receipts.v1".to_string(),
            "canic.core.cycles.tracker.v1".to_string(),
            "canic.core.cycles.topup_events.v1".to_string(),
            "canic.core.cycles.funding_ledger.v1".to_string(),
            "canic.core.log.entries.v1".to_string(),
            "canic.core.intent.meta.v1".to_string(),
            "canic.core.intent.records.v1".to_string(),
            "canic.core.intent.totals.v1".to_string(),
            "canic.core.intent.pending.v1".to_string(),
            "canic.core.intent.receipt_backed_records.v1".to_string(),
            "canic.core.intent.expiry_index.v1".to_string(),
            "canic.core.caller_authority.header.v1".to_string(),
            "canic.core.application_receipt.eligibility.v1".to_string(),
            "canic.core.placement.acknowledgement_index.v1".to_string(),
            "canic.core.sharding.registry.v1".to_string(),
            "canic.core.sharding.assignments.v1".to_string(),
            "canic.core.caller_authority.rows.v1".to_string(),
            "canic.core.async_job_recovery.v1".to_string()
        ])
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
        allocation_keys(&contract.allocations),
        sorted_keys(vec![
            "canic.control_plane.template.manifests.v1".to_string(),
            "canic.control_plane.template.chunk_sets.v1".to_string(),
            "canic.control_plane.template.chunk_refs.v1".to_string(),
            "canic.control_plane.template.chunk_payloads.v1".to_string(),
            "canic.control_plane.root.wasm_store.state.v1".to_string(),
            "canic.control_plane.root.fleet_registry_mirror.v1".to_string(),
            "canic.control_plane.root.component.registry_state.v1".to_string(),
            "canic.control_plane.root.component.allocations.v1".to_string(),
            "canic.control_plane.root.component.registry_entries.v1".to_string(),
            "canic.control_plane.root.component.principal_index.v1".to_string(),
            "canic.control_plane.root.component.subtree_removal_history.v1".to_string(),
            "canic.control_plane.root.component.draining.v1".to_string(),
            "canic.control_plane.root.canister_inventory.assets.v1".to_string(),
            "canic.control_plane.root.canister_pool.state.v1".to_string(),
            "canic.control_plane.root.canister_pool.handoff_receipts.v1".to_string(),
            "canic.control_plane.root.component_provisioning.operations.v1".to_string(),
            "canic.control_plane.root.component_provisioning.placements.v1".to_string(),
            "canic.control_plane.root.component_provisioning.state.v1".to_string(),
            "canic.core.runtime.canister_children.v1".to_string(),
            "canic.core.runtime.bindings.v1".to_string(),
            "canic.core.fleet.state.v1".to_string(),
            "canic.core.fleet.activation.v1".to_string(),
            "canic.core.replay.receipts.v1".to_string(),
            "canic.core.cycles.tracker.v1".to_string(),
            "canic.core.cycles.topup_events.v1".to_string(),
            "canic.core.cycles.funding_ledger.v1".to_string(),
            "canic.core.cycles.icp_refill_records.v1".to_string(),
            "canic.core.log.entries.v1".to_string(),
            "canic.core.intent.meta.v1".to_string(),
            "canic.core.intent.records.v1".to_string(),
            "canic.core.intent.totals.v1".to_string(),
            "canic.core.intent.pending.v1".to_string(),
            "canic.core.intent.receipt_backed_records.v1".to_string(),
            "canic.core.intent.expiry_index.v1".to_string(),
            "canic.core.application_receipt.eligibility.v1".to_string(),
            "canic.core.placement.acknowledgement_index.v1".to_string(),
            "canic.core.authority_restore.fence.v1".to_string(),
            "canic.core.async_job_recovery.v1".to_string(),
            "canic.control_plane.root.funding.v1".to_string(),
            "canic.control_plane.root.admission.v1".to_string()
        ])
    );
    assert_eq!(
        contract
            .allocations
            .iter()
            .filter_map(|allocation| match allocation.key {
                StateAllocationKey::CoreDelegatedTokenIssuerState
                | StateAllocationKey::CoreLocalApplicationAuthorizationState
                | StateAllocationKey::CoreRootDelegationState => Some(allocation.key),
                _ => None,
            })
            .collect::<BTreeSet<_>>(),
        BTreeSet::new()
    );
}

#[test]
fn built_in_wasm_store_owns_template_gc_and_fixture_ids() {
    let resolution = resolve_role_contract(RoleContractInput {
        source: RoleContractSource::BuiltIn(BuiltInRoleKind::WasmStore),
        declared_features: BTreeSet::from([CanicFeatureKey::WasmStoreCanister]),
        default_features_enabled: false,
    });
    let RoleContractResolution::Resolved { contract } = resolution else {
        panic!("built-in wasm_store contract should resolve");
    };

    assert_eq!(
        allocation_keys(&contract.allocations),
        sorted_keys(vec![
            "canic.control_plane.template.manifests.v1".to_string(),
            "canic.control_plane.template.chunk_sets.v1".to_string(),
            "canic.control_plane.template.chunk_refs.v1".to_string(),
            "canic.control_plane.template.chunk_payloads.v1".to_string(),
            "canic.control_plane.wasm_store.gc_state.v1".to_string(),
            "canic.core.runtime.canister_children.v1".to_string(),
            "canic.core.runtime.bindings.v1".to_string(),
            "canic.core.fleet.state.v1".to_string(),
            "canic.core.fleet.activation.v1".to_string(),
            "canic.core.replay.receipts.v1".to_string(),
            "canic.core.cycles.tracker.v1".to_string(),
            "canic.core.cycles.topup_events.v1".to_string(),
            "canic.core.cycles.funding_ledger.v1".to_string(),
            "canic.core.log.entries.v1".to_string(),
            "canic.core.intent.meta.v1".to_string(),
            "canic.core.intent.records.v1".to_string(),
            "canic.core.intent.totals.v1".to_string(),
            "canic.core.intent.pending.v1".to_string(),
            "canic.core.intent.receipt_backed_records.v1".to_string(),
            "canic.core.intent.expiry_index.v1".to_string(),
            "canic.core.application_receipt.eligibility.v1".to_string(),
            "canic.core.placement.acknowledgement_index.v1".to_string(),
            "canic.core.async_job_recovery.v1".to_string(),
            "canic.control_plane.fixture_store.v1".to_string()
        ])
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

    assert_eq!(
        allocation_keys(&contract.allocations),
        sorted_keys(vec![
            "canic.control_plane.fleet_coordinator.registry.v1".to_string(),
            "canic.core.authority_restore.fence.v1".to_string(),
            "canic.control_plane.fleet_coordinator.funding.v1".to_string(),
            "canic.control_plane.fleet_admission.v1".to_string()
        ])
    );
    assert!(contract.allocations.iter().all(|allocation| !matches!(
        allocation.key,
        StateAllocationKey::CoreDelegatedTokenIssuerState
            | StateAllocationKey::CoreLocalApplicationAuthorizationState
            | StateAllocationKey::CoreRootDelegationState
    )));
    assert_eq!(
        contract.required_features,
        BTreeSet::from([CanicFeatureKey::FleetCoordinatorCanister])
    );
}

fn allocation_keys(allocations: &[super::ResolvedStateAllocation]) -> Vec<String> {
    let mut ids = allocations
        .iter()
        .flat_map(|allocation| allocation.memory_keys.iter())
        .cloned()
        .collect::<Vec<_>>();
    ids.sort_unstable();
    ids
}

fn placement_allocation_keys(allocations: &[super::ResolvedStateAllocation]) -> Vec<String> {
    let selected: Vec<_> = allocations
        .iter()
        .filter(|allocation| {
            matches!(
                allocation.key,
                StateAllocationKey::PlacementScalingRegistry
                    | StateAllocationKey::PlacementIndexRegistry
                    | StateAllocationKey::ShardingRegistry
                    | StateAllocationKey::ShardingAssignments
            )
        })
        .cloned()
        .collect();
    allocation_keys(&selected)
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

fn sorted_keys(mut keys: Vec<String>) -> Vec<String> {
    keys.sort_unstable();
    keys
}
