//! Module: fleet_install_input::tests
//!
//! Responsibility: qualify strict document decoding and trusted placement/funding resolution.
//! Does not own: immutable Fleet plan persistence or install workflow mutation ordering.

use super::*;
use crate::test_support::temp_dir;
use canic_core::ids::{
    COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES,
    FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES, FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES,
};
use std::fs;

use ic_query::subnet_catalog::{
    CatalogValidationContext, ClassificationSource, DEFAULT_CATALOG_MAX_FUTURE_SKEW_SECONDS,
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, GeographicScope, MAINNET_NETWORK,
    MAINNET_REGISTRY_CANISTER_ID, RawSubnetCatalog, RoutingRange,
    SubnetCatalogRegistryRecordEvidence, SubnetCatalogRegistryRecordSubject,
    SubnetCatalogRegistryValueEncoding, SubnetCatalogRoutingSource, SubnetInfo,
    UncertifiedCatalogCollection, ValidatedSubnetCatalog,
};

const FIDUCIARY_SUBNET: &str = "pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae";
const EUROPEAN_SUBNET: &str = "bkfrj-6k62g-dycql-7h53p-atvkj-zg4to-gaogh-netha-ptybj-ntsgw-rqe";
const FIXTURE_CANISTER: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
const FIXTURE_FETCHED_AT: &str = "2026-06-26T00:00:00Z";
const FIXTURE_NOW_UNIX_SECS: u64 = 1_782_432_100;
const DISPOSABLE_ROOT_DELETION_PROOF_INPUT: &str =
    include_str!("../../../../deployments/0.100-root-deletion-proof.toml");
const PLAYGROUND_INPUT: &str = include_str!("../../../../deployments/demos/playground-ic.toml");

#[test]
fn operator_balance_validity_is_strict_and_bounded() {
    let input = document(CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    });

    let fresh = resolve_operator(
        &input.operator,
        BuildNetwork::Local,
        input.operator.valid_until_unix_secs - 1,
    )
    .expect("balance is fresh immediately before exclusive expiry");
    assert!(fresh.balance_fresh);

    assert!(matches!(
        resolve_operator(
            &input.operator,
            BuildNetwork::Local,
            input.operator.valid_until_unix_secs,
        ),
        Err(FleetInstallInputError::StaleOperatorBalance { .. })
    ));
    assert!(matches!(
        resolve_operator(
            &input.operator,
            BuildNetwork::Local,
            input.operator.observed_at_unix_secs - 1,
        ),
        Err(FleetInstallInputError::InvalidOperatorEvidence {
            field: "observed_at_unix_secs"
        })
    ));
}

#[test]
fn disposable_root_deletion_proof_input_resolves_one_bounded_mainnet_root() {
    let document: FleetInstallInputDocument =
        toml::from_slice(DISPOSABLE_ROOT_DELETION_PROOF_INPUT.as_bytes())
            .expect("decode disposable root-deletion proof input");
    let catalog = catalog(vec![info(
        FIDUCIARY_SUBNET,
        SubnetKind::Application,
        SubnetSpecialization::Fiduciary,
        "fiduciary",
    )]);

    let resolved = resolve_document(&document, BuildNetwork::Ic, Some(&catalog))
        .expect("resolve disposable root-deletion proof input");

    assert_eq!(
        resolved.coordinator.coordinator_subnet,
        subnet(FIDUCIARY_SUBNET)
    );
    assert_eq!(
        resolved.coordinator.creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 100_000_000_000_000
        }
    );
    let [root] = resolved.fleet_subnet_roots.as_slice() else {
        panic!("proof input must resolve exactly one Fleet Subnet Root");
    };
    assert_eq!(root.placement_subnet, subnet(FIDUCIARY_SUBNET));
    assert_eq!(root.component_admissions.len(), 2);
    assert_eq!(root.limits.maximum_component_instances, 2);
    assert_eq!(root.limits.maximum_group_placements, 0);
    assert_eq!(root.limits.canister_pool.minimum_size, 1);
    assert_eq!(root.limits.canister_pool.maximum_size, 1);
    assert_eq!(
        root.limits.canister_pool.canister_cycles,
        Cycles::new(1_000_000_000_000)
    );
    assert!(root.canister_pool_imports.is_empty());
    assert_eq!(
        root.root_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 30_000_000_000_000
        }
    );
    assert_eq!(
        root.wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 10_000_000_000_000
        }
    );
}

#[test]
fn playground_input_resolves_one_reusable_mainnet_root() {
    let document: FleetInstallInputDocument =
        toml::from_slice(PLAYGROUND_INPUT.as_bytes()).expect("decode Playground input");
    let catalog = catalog(vec![info(
        FIDUCIARY_SUBNET,
        SubnetKind::Application,
        SubnetSpecialization::Fiduciary,
        "fiduciary",
    )]);

    let resolved = resolve_document(&document, BuildNetwork::Ic, Some(&catalog))
        .expect("resolve Playground input");

    assert_eq!(
        resolved.coordinator.coordinator_subnet,
        subnet(FIDUCIARY_SUBNET)
    );
    let [root] = resolved.fleet_subnet_roots.as_slice() else {
        panic!("Playground input must resolve exactly one Fleet Subnet Root");
    };
    assert_eq!(root.placement_subnet, subnet(FIDUCIARY_SUBNET));
    assert_eq!(
        root.component_admissions,
        vec![RootComponentAdmissionInput {
            component_spec: "playground".parse().expect("valid Component Spec ID"),
            maximum_root_instances: 1,
        }]
    );
    assert_eq!(root.limits.maximum_component_instances, 1);
    assert_eq!(root.limits.maximum_group_placements, 16);
    assert_eq!(root.limits.canister_pool.minimum_size, 5);
    assert_eq!(root.limits.canister_pool.maximum_size, 5);
    assert_eq!(
        root.limits.canister_pool.canister_cycles,
        Cycles::new(500_000_000_000)
    );
    assert!(root.canister_pool_imports.is_empty());
    assert_eq!(
        root.limits.cycles_funding.maximum_cycles,
        Cycles::new(15_000_000_000_000)
    );
    assert_eq!(
        root.root_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 30_000_000_000_000
        }
    );
    assert_eq!(
        root.wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 10_000_000_000_000
        }
    );
}

#[test]
fn local_document_resolves_exact_explicit_placement_and_cycles() {
    let application_subnet = subnet_text(7);
    let document = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });

    let resolved =
        resolve_document(&document, BuildNetwork::Local, None).expect("resolve local input");

    assert_eq!(
        resolved.coordinator.coordinator_subnet,
        subnet(&application_subnet)
    );
    assert_eq!(
        resolved.coordinator.creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 100_000_000_000_000
        }
    );
    assert_eq!(
        resolved.coordinator.root_funding,
        Some(crate::test_support::coordinator_root_funding_policy())
    );
    assert_eq!(resolved.fleet_subnet_roots.len(), 1);
    assert_eq!(
        resolved.fleet_subnet_roots[0]
            .limits
            .maximum_group_placements,
        16
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0].root_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 30_000_000_000_000
        }
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0].wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 10_000_000_000_000
        }
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0].funding.root_funding,
        crate::test_support::fleet_subnet_root_funding_authority().root_funding
    );
    let icp_refill = resolved.fleet_subnet_roots[0]
        .funding
        .icp_refill
        .as_ref()
        .expect("root ICP policy");
    assert_eq!(icp_refill.maximum_refill_e8s, 200_000_000);
    assert_eq!(
        icp_refill
            .automatic
            .as_ref()
            .expect("automatic root ICP policy")
            .emergency_threshold,
        Cycles::new(5_000_000_000_000)
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0].component_admissions,
        vec![RootComponentAdmissionInput {
            component_spec: "users".parse().expect("valid Component Spec ID"),
            maximum_root_instances: 8,
        }]
    );
}

#[test]
fn protected_funding_policies_are_required_for_every_planned_root() {
    let selector = CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    };
    let mut missing_coordinator = document(selector.clone());
    missing_coordinator.coordinator.root_funding = None;
    assert!(matches!(
        resolve_document(&missing_coordinator, BuildNetwork::Local, None),
        Err(FleetInstallInputError::MissingCoordinatorRootFundingPolicy)
    ));

    let mut missing_root = document(selector);
    let expected_subnet = subnet(&missing_root.fleet_subnet_roots[0].placement_subnet);
    missing_root.fleet_subnet_roots[0].root_funding = None;
    assert!(matches!(
        resolve_document(&missing_root, BuildNetwork::Local, None),
        Err(FleetInstallInputError::MissingRootFundingPolicy { placement_subnet })
            if placement_subnet == expected_subnet
    ));
}

#[test]
fn protected_funding_policy_rejects_underfloor_and_unfundable_values() {
    let selector = CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    };

    let mut coordinator_reserve = document(selector.clone());
    coordinator_reserve
        .coordinator
        .root_funding
        .as_mut()
        .expect("Coordinator policy")
        .minimum_reserve_cycles =
        Cycles::new(COORDINATOR_ROOT_FUNDING_EXECUTION_RESERVE_FLOOR_CYCLES - 1);
    assert_invalid_policy(
        &coordinator_reserve,
        "coordinator.root_funding.minimum_reserve_cycles",
    );

    let mut request_floor = document(selector.clone());
    request_floor.fleet_subnet_roots[0]
        .root_funding
        .as_mut()
        .expect("root policy")
        .request_threshold = Cycles::new(FLEET_SUBNET_ROOT_FUNDING_REQUEST_FLOOR_CYCLES - 1);
    assert_invalid_policy(&request_floor, ".root_funding.request_threshold");

    let mut root_target = document(selector.clone());
    let root_policy = root_target.fleet_subnet_roots[0]
        .root_funding
        .as_mut()
        .expect("root policy");
    root_policy.target_balance = root_policy.request_threshold.clone();
    assert_invalid_policy(&root_target, ".root_funding.target_balance");

    let mut root_budget = document(selector.clone());
    root_budget.fleet_subnet_roots[0]
        .root_funding
        .as_mut()
        .expect("root policy")
        .maximum_cycles = Cycles::new(1_000_000_000_000);
    assert_invalid_policy(&root_budget, ".root_funding.maximum_cycles");

    let mut fleet_budget = document(selector);
    fleet_budget
        .coordinator
        .root_funding
        .as_mut()
        .expect("Coordinator policy")
        .maximum_cycles = Cycles::new(1_000_000_000_000);
    assert!(matches!(
        resolve_document(&fleet_budget, BuildNetwork::Local, None),
        Err(FleetInstallInputError::FundingProfileMinimum { ref owner, .. })
            if owner == "Fleet window maximum"
    ));
}

#[test]
fn automatic_icp_policy_rejects_invalid_thresholds_caps_and_unsafe_ic_overrides() {
    let selector = CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    };

    let mut emergency_floor = document(selector.clone());
    emergency_floor.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .automatic
        .as_mut()
        .expect("automatic policy")
        .emergency_threshold = Cycles::new(FLEET_SUBNET_ROOT_ICP_REFILL_FLOOR_CYCLES - 1);
    assert_invalid_policy(
        &emergency_floor,
        ".icp_refill.automatic.emergency_threshold",
    );

    let mut overlapping_threshold = document(selector.clone());
    let request_threshold = overlapping_threshold.fleet_subnet_roots[0]
        .root_funding
        .as_ref()
        .expect("root policy")
        .request_threshold
        .clone();
    overlapping_threshold.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .automatic
        .as_mut()
        .expect("automatic policy")
        .emergency_threshold = request_threshold.clone();
    assert_invalid_policy(
        &overlapping_threshold,
        ".icp_refill.automatic.emergency_threshold",
    );

    let mut low_target = document(selector.clone());
    low_target.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .automatic
        .as_mut()
        .expect("automatic policy")
        .target_balance = request_threshold;
    assert_invalid_policy(&low_target, ".icp_refill.automatic.target_balance");

    let mut refill_budget = document(selector.clone());
    refill_budget.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .maximum_refill_e8s = 99_999_999;
    assert_invalid_policy(&refill_budget, ".icp_refill.maximum_refill_e8s");

    let application_subnet = subnet_text(7);
    let public_catalog = catalog(vec![info(
        &application_subnet,
        SubnetKind::Application,
        SubnetSpecialization::None,
        "application",
    )]);
    let mut override_input = document(selector);
    override_input.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .ledger_canister_id = Some(Principal::from_slice(&[44; 29]).to_text());
    assert!(matches!(
        resolve_document(&override_input, BuildNetwork::Ic, Some(&public_catalog)),
        Err(FleetInstallInputError::UnsafeIcpRefillOverride {
            field: "fleet_subnet_roots.icp_refill.ledger_canister_id"
        })
    ));
    override_input.fleet_subnet_roots[0]
        .icp_refill
        .as_mut()
        .expect("ICP policy")
        .allow_ic_system_canister_overrides = true;
    resolve_document(&override_input, BuildNetwork::Ic, Some(&public_catalog))
        .expect("explicit IC override safety acknowledgement");
}

#[test]
fn protected_policy_changes_canonical_fleet_input_identity() {
    let selector = CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    };
    let baseline = resolve_document(&document(selector.clone()), BuildNetwork::Local, None)
        .expect("baseline policy");
    let mut changed_document = document(selector);
    changed_document.fleet_subnet_roots[0]
        .root_funding
        .as_mut()
        .expect("root policy")
        .cooldown_secs += 1;
    let changed =
        resolve_document(&changed_document, BuildNetwork::Local, None).expect("changed policy");

    assert_ne!(baseline.canonical_sha256, changed.canonical_sha256);
}

#[test]
fn local_document_preserves_explicit_component_group_placement_ordinals() {
    let application_subnet = subnet_text(7);
    let mut document = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet,
        acknowledge_fiduciary_cost: false,
    });
    document.fleet_subnet_roots[0]
        .component_group_placements
        .insert("project_cells".parse().expect("deployment ID"), vec![0, 2]);

    let resolved = resolve_document(&document, BuildNetwork::Local, None)
        .expect("resolve explicit placement assignments");
    assert_eq!(
        resolved.fleet_subnet_roots[0]
            .component_group_placements
            .iter()
            .map(|assignment| (assignment.deployment.as_str(), assignment.ordinal))
            .collect::<Vec<_>>(),
        vec![("project_cells", 0), ("project_cells", 2)]
    );
}

#[test]
fn group_placement_ceiling_is_required_and_zero_remains_an_explicit_fence() {
    let missing = input_toml().replace("maximum_group_placements = 16\n", "");
    assert!(toml::from_slice::<FleetInstallInputDocument>(missing.as_bytes()).is_err());

    let mut document = document(CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    });
    document.fleet_subnet_roots[0]
        .limits
        .maximum_group_placements = 0;
    let resolved = resolve_document(&document, BuildNetwork::Local, None)
        .expect("zero is an explicit root-ineligibility fence");
    assert_eq!(
        resolved.fleet_subnet_roots[0]
            .limits
            .maximum_group_placements,
        0
    );
}

#[test]
fn public_profile_selects_one_exact_application_subnet() {
    let application_subnet = subnet_text(7);
    let catalog = catalog(vec![
        info(
            FIDUCIARY_SUBNET,
            SubnetKind::Application,
            SubnetSpecialization::Fiduciary,
            "fiduciary",
        ),
        info(
            &application_subnet,
            SubnetKind::Application,
            SubnetSpecialization::None,
            "application",
        ),
        info(
            EUROPEAN_SUBNET,
            SubnetKind::Application,
            SubnetSpecialization::European,
            "european",
        ),
    ]);
    let profile = resolve_document(
        &document(CoordinatorSubnetSelector::Profile {
            profile: "application".to_string(),
            acknowledge_fiduciary_cost: false,
        }),
        BuildNetwork::Ic,
        Some(&catalog),
    )
    .expect("resolve profile");
    assert_eq!(
        profile.coordinator.coordinator_subnet,
        subnet(&application_subnet)
    );
}

#[test]
fn recommended_coordinator_selector_is_a_hard_decode_cut() {
    let input = input_toml().replace(
        &format!(
            "kind = \"explicit\"\nsubnet = \"{}\"\nacknowledge_fiduciary_cost = false",
            subnet_text(7)
        ),
        "kind = \"recommended\"",
    );
    assert!(toml::from_slice::<FleetInstallInputDocument>(input.as_bytes()).is_err());
}

#[test]
fn catalog_snapshot_authority_excludes_transient_acquisition_provenance() {
    let catalog = catalog(vec![info(
        FIDUCIARY_SUBNET,
        SubnetKind::Application,
        SubnetSpecialization::Fiduciary,
        "fiduciary",
    )]);
    let refreshed = CatalogLoadOutcome {
        path: PathBuf::from("refreshed-subnet-catalog.json"),
        catalog: catalog.clone(),
        disposition: CacheDisposition::RefreshedMissing,
    };
    let cached = CatalogLoadOutcome {
        path: PathBuf::from("cached-subnet-catalog.json"),
        catalog,
        disposition: CacheDisposition::CacheHit,
    };

    let (refreshed_authority, refreshed_acquisition) =
        resolve_catalog_evidence(BuildNetwork::Ic, Some(&refreshed))
            .expect("resolve refreshed catalog evidence");
    let (cached_authority, cached_acquisition) =
        resolve_catalog_evidence(BuildNetwork::Ic, Some(&cached))
            .expect("resolve cached catalog evidence");

    assert_eq!(refreshed_authority, cached_authority);
    let authority_json =
        serde_json::to_value(&refreshed_authority).expect("serialize catalog authority");
    assert!(authority_json.get("cache_path").is_none());
    assert!(authority_json.get("cache_disposition").is_none());
    assert!(authority_json.get("collected_at").is_none());
    assert_eq!(
        refreshed_acquisition,
        FleetInstallCatalogAcquisitionV1::ValidatedCache {
            cache_path: "refreshed-subnet-catalog.json".to_string(),
            cache_disposition: "refreshed_missing".to_string(),
            collected_at: FIXTURE_FETCHED_AT.to_string(),
        }
    );
    assert_eq!(
        cached_acquisition,
        FleetInstallCatalogAcquisitionV1::ValidatedCache {
            cache_path: "cached-subnet-catalog.json".to_string(),
            cache_disposition: "cache_hit".to_string(),
            collected_at: FIXTURE_FETCHED_AT.to_string(),
        }
    );
}

#[test]
fn public_resolution_enforces_trusted_eligibility_and_funding_method() {
    let system_subnet = subnet_text(8);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: system_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });
    input.coordinator.creation_funding = CreationFundingDocument::Icp { e8s: 100_000_000 };
    input.fleet_subnet_roots[0].placement_subnet = system_subnet.clone();
    input.fleet_subnet_roots[0].root_creation_funding =
        CreationFundingDocument::Icp { e8s: 100_000_000 };
    input.fleet_subnet_roots[0].wasm_store_creation_funding =
        CreationFundingDocument::Icp { e8s: 100_000_000 };
    let system_catalog = catalog(vec![info(
        &system_subnet,
        SubnetKind::System,
        SubnetSpecialization::None,
        "system",
    )]);

    let resolved = resolve_document(&input, BuildNetwork::Ic, Some(&system_catalog))
        .expect("resolve restricted System Subnet");
    assert_eq!(
        resolved.coordinator.creation_funding,
        PlannedCanisterCreationFunding::Icp { e8s: 100_000_000 }
    );

    input.coordinator.creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(1),
    };
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&system_catalog)),
        Err(FleetInstallInputError::FundingMismatch { .. })
    ));
}

#[test]
fn nonpublic_network_rejects_profile_selectors_and_icp_funding() {
    assert!(matches!(
        resolve_document(
            &document(CoordinatorSubnetSelector::Profile {
                profile: "application".to_string(),
                acknowledge_fiduciary_cost: false,
            }),
            BuildNetwork::Local,
            None
        ),
        Err(FleetInstallInputError::TrustedMetadataRequired { .. })
    ));

    let application_subnet = subnet_text(7);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet,
        acknowledge_fiduciary_cost: false,
    });
    input.coordinator.creation_funding = CreationFundingDocument::Icp { e8s: 1 };
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Local, None),
        Err(FleetInstallInputError::NonPublicFunding { .. })
    ));

    input.coordinator.creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(0),
    };
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Local, None),
        Err(FleetInstallInputError::NonPositiveCreationFunding { .. })
    ));
}

#[test]
fn public_resolution_rejects_ineligible_and_ambiguous_subnets() {
    let cloud_subnet = subnet_text(9);
    let input = document(CoordinatorSubnetSelector::Explicit {
        subnet: cloud_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });
    let cloud_catalog = catalog(vec![info(
        &cloud_subnet,
        SubnetKind::CloudEngine,
        SubnetSpecialization::None,
        "cloud",
    )]);
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&cloud_catalog)),
        Err(FleetInstallInputError::IneligibleSubnet { .. })
    ));

    let ambiguous_catalog = catalog(vec![
        info(
            &subnet_text(10),
            SubnetKind::Application,
            SubnetSpecialization::None,
            "application",
        ),
        info(
            &subnet_text(11),
            SubnetKind::Application,
            SubnetSpecialization::None,
            "application",
        ),
    ]);
    assert!(matches!(
        resolve_document(
            &document(CoordinatorSubnetSelector::Profile {
                profile: "application".to_string(),
                acknowledge_fiduciary_cost: false,
            }),
            BuildNetwork::Ic,
            Some(&ambiguous_catalog)
        ),
        Err(FleetInstallInputError::AmbiguousSubnetSelector { matches: 2, .. })
    ));
}

#[test]
fn loader_rejects_unknown_fields_unsupported_schema_and_symlink() {
    let root = temp_dir("fleet-install-input-loader");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("fleet-install.toml");
    fs::write(
        &path,
        input_toml().replace("schema_version = 1", "schema_version = 2"),
    )
    .expect("write input");
    assert!(matches!(
        load_document(&path),
        Err(FleetInstallInputError::UnsupportedSchemaVersion { actual: 2 })
    ));

    fs::write(&path, format!("{}\nunknown = true\n", input_toml())).expect("write unknown field");
    assert!(matches!(
        load_document(&path),
        Err(FleetInstallInputError::Decode { .. })
    ));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        fs::write(&path, input_toml()).expect("write valid input");
        let link = root.join("linked.toml");
        symlink(&path, &link).expect("create symlink");
        assert!(matches!(
            load_document(&link),
            Err(FleetInstallInputError::NotRegular { .. })
        ));
    }

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn loader_decodes_the_document_shape_and_cycle_shorthand() {
    let root = temp_dir("fleet-install-input-shape");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("fleet-install.toml");
    fs::write(&path, input_toml()).expect("write input");

    let document = load_document(&path).expect("load input");

    assert_eq!(document.schema_version, 1);
    assert_eq!(
        document.coordinator.creation_funding,
        CreationFundingDocument::Cycles {
            cycles: Cycles::new(100_000_000_000_000)
        }
    );
    assert_eq!(
        document.fleet_subnet_roots[0]
            .limits
            .cycles_funding
            .maximum_cycles,
        Cycles::new(10_000_000_000_000)
    );
    assert_eq!(
        document.fleet_subnet_roots[0].root_creation_funding,
        CreationFundingDocument::Cycles {
            cycles: Cycles::new(30_000_000_000_000)
        }
    );
    assert_eq!(
        document.fleet_subnet_roots[0].wasm_store_creation_funding,
        CreationFundingDocument::Cycles {
            cycles: Cycles::new(10_000_000_000_000)
        }
    );
    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn loader_hard_cuts_the_ambiguous_root_creation_funding_field() {
    let root = temp_dir("fleet-install-input-obsolete-funding");
    fs::create_dir_all(&root).expect("create temp root");
    let path = root.join("fleet-install.toml");
    let source = input_toml().replace(
        "fleet_subnet_roots.root_creation_funding",
        "fleet_subnet_roots.creation_funding",
    );
    fs::write(&path, source).expect("write obsolete input");

    assert!(matches!(
        load_document(&path),
        Err(FleetInstallInputError::Decode { .. })
    ));

    fs::remove_dir_all(root).expect("remove temp root");
}

#[test]
fn pool_imports_are_unique_across_fleet_subnet_roots() {
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    });
    let imported = Principal::from_slice(&[44; 29]).to_text();
    input.fleet_subnet_roots[0].canister_pool.imports = vec![imported.clone()];
    let mut second = input.fleet_subnet_roots[0].clone();
    second.placement_subnet = subnet_text(8);
    second.canister_pool.imports = vec![imported];
    input.fleet_subnet_roots.push(second);

    assert!(matches!(
        resolve_document(&input, BuildNetwork::Local, None),
        Err(FleetInstallInputError::InvalidCanisterPool { .. })
    ));
}

#[test]
fn pool_import_rejects_reserved_principals() {
    assert!(matches!(
        parse_canister(
            "fleet_subnet_roots.canister_pool.imports",
            &Principal::management_canister().to_text(),
        ),
        Err(FleetInstallInputError::InvalidCanister { .. })
    ));
}

#[test]
fn pool_policy_rejects_invalid_capacity_and_funding() {
    let selector = CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    };
    let mut cases = Vec::new();

    let mut zero_minimum = document(selector.clone());
    zero_minimum.fleet_subnet_roots[0]
        .canister_pool
        .minimum_size = 0;
    cases.push(zero_minimum);

    let mut inverted_bounds = document(selector.clone());
    inverted_bounds.fleet_subnet_roots[0]
        .canister_pool
        .minimum_size = 11;
    cases.push(inverted_bounds);

    let mut imports_exceed_maximum = document(selector.clone());
    imports_exceed_maximum.fleet_subnet_roots[0]
        .canister_pool
        .imports = (1_u8..=11)
        .map(|byte| Principal::from_slice(&[byte; 29]).to_text())
        .collect();
    cases.push(imports_exceed_maximum);

    let mut zero_cycles = document(selector);
    zero_cycles.fleet_subnet_roots[0]
        .canister_pool
        .canister_cycles = Cycles::new(0);
    cases.push(zero_cycles);

    for input in cases {
        assert!(matches!(
            resolve_document(&input, BuildNetwork::Local, None),
            Err(FleetInstallInputError::InvalidCanisterPool { .. })
        ));
    }
}

#[test]
fn ic_pool_imports_require_exact_root_subnet_routing() {
    let root_subnet = subnet_text(7);
    let other_subnet = subnet_text(8);
    let imported = Principal::from_slice(&[44; 29]);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: root_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });
    input.fleet_subnet_roots[0].canister_pool.imports = vec![imported.to_text()];
    let subnets = vec![
        info(
            &root_subnet,
            SubnetKind::Application,
            SubnetSpecialization::None,
            "root",
        ),
        info(
            &other_subnet,
            SubnetKind::Application,
            SubnetSpecialization::None,
            "other",
        ),
    ];
    let catalog = catalog_with_ranges(
        subnets.clone(),
        vec![RoutingRange {
            start_canister_id: imported.to_text(),
            end_canister_id: imported.to_text(),
            subnet_principal: root_subnet,
        }],
    );

    resolve_document(&input, BuildNetwork::Ic, Some(&catalog))
        .expect("pool import routes to its Fleet Subnet Root");

    let contradictory_catalog = catalog_with_ranges(
        subnets,
        vec![RoutingRange {
            start_canister_id: imported.to_text(),
            end_canister_id: imported.to_text(),
            subnet_principal: other_subnet,
        }],
    );
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&contradictory_catalog)),
        Err(FleetInstallInputError::ImportedCanisterSubnetMismatch { .. })
    ));
}

#[test]
fn ic_pool_imports_fail_closed_without_routing_evidence() {
    let root_subnet = subnet_text(7);
    let imported = Principal::from_slice(&[44; 29]);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: root_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });
    input.fleet_subnet_roots[0].canister_pool.imports = vec![imported.to_text()];
    let catalog = catalog(vec![info(
        &root_subnet,
        SubnetKind::Application,
        SubnetSpecialization::None,
        "root",
    )]);

    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&catalog)),
        Err(FleetInstallInputError::ImportedCanisterRoute { .. })
    ));
}

#[test]
fn funding_profile_must_match_the_resolved_physical_topology() {
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: subnet_text(7),
        acknowledge_fiduciary_cost: false,
    });
    input.fleet_subnet_roots[0].placement_subnet = subnet_text(8);

    assert!(matches!(
        resolve_document(&input, BuildNetwork::Local, None),
        Err(FleetInstallInputError::FundingProfileMismatch {
            configured: FleetFundingProfile::SingleSubnet,
            resolved: FleetFundingProfile::MultiSubnet,
        })
    ));

    input.funding_profile = FleetFundingProfile::PreviewMultiSubnet;
    let coordinator_policy = input
        .coordinator
        .root_funding
        .as_mut()
        .expect("Coordinator funding policy");
    coordinator_policy.minimum_reserve_cycles = Cycles::new(80 * TRILLION_CYCLES);
    coordinator_policy.maximum_automatic_grants = 2;
    coordinator_policy.maximum_automatic_cycles = Cycles::new(60 * TRILLION_CYCLES);
    input.coordinator.creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(140 * TRILLION_CYCLES),
    };
    let root = &mut input.fleet_subnet_roots[0];
    let root_policy = root.root_funding.as_mut().expect("Root funding policy");
    root_policy.maximum_automatic_grants = 2;
    root_policy.maximum_automatic_cycles = Cycles::new(60 * TRILLION_CYCLES);
    root.icp_refill = None;

    let resolved = resolve_document(&input, BuildNetwork::Local, None)
        .expect("bounded preview multi-Subnet funding profile");
    assert_eq!(
        resolved.funding_profile,
        FleetFundingProfile::PreviewMultiSubnet
    );
    assert_eq!(
        resolved.coordinator.creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 140 * TRILLION_CYCLES
        }
    );
    assert_eq!(
        resolved
            .coordinator
            .root_funding
            .expect("Coordinator funding policy")
            .minimum_reserve_cycles,
        Cycles::new(80 * TRILLION_CYCLES)
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0]
            .funding
            .root_funding
            .maximum_automatic_cycles,
        Cycles::new(60 * TRILLION_CYCLES)
    );
    assert!(resolved.fleet_subnet_roots[0].funding.icp_refill.is_none());
}

#[test]
fn fiduciary_acknowledgement_is_exact_per_placement_and_retained_with_warning() {
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: FIDUCIARY_SUBNET.to_string(),
        acknowledge_fiduciary_cost: false,
    });
    input.fleet_subnet_roots[0].placement_subnet = FIDUCIARY_SUBNET.to_string();
    let fiduciary_catalog = catalog(vec![info(
        FIDUCIARY_SUBNET,
        SubnetKind::Application,
        SubnetSpecialization::Fiduciary,
        "fiduciary",
    )]);

    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&fiduciary_catalog)),
        Err(FleetInstallInputError::FiduciaryCostAcknowledgementRequired { ref owner, .. })
            if owner == "Fleet Coordinator"
    ));

    input.coordinator.subnet = CoordinatorSubnetSelector::Explicit {
        subnet: FIDUCIARY_SUBNET.to_string(),
        acknowledge_fiduciary_cost: true,
    };
    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&fiduciary_catalog)),
        Err(FleetInstallInputError::FiduciaryCostAcknowledgementRequired { ref owner, .. })
            if owner.starts_with("Fleet Subnet Root")
    ));

    input.fleet_subnet_roots[0].acknowledge_fiduciary_cost = true;
    let resolved = resolve_document(&input, BuildNetwork::Ic, Some(&fiduciary_catalog))
        .expect("both exact Fiduciary acknowledgements admit the plan");
    let coordinator_evidence = &resolved.coordinator.placement_cost;
    assert!(coordinator_evidence.acknowledge_fiduciary_cost);
    assert!(coordinator_evidence.catalog_sha256.is_some());
    let warning = coordinator_evidence
        .warning
        .as_deref()
        .expect("warning retained");
    assert!(warning.contains("Fleet Coordinator"));
    assert!(warning.contains(FIDUCIARY_SUBNET));
    assert!(warning.contains("node_count=13"));
    assert!(warning.contains("cost_multiplier=13/13"));
    assert!(warning.contains("creation_funding=100000000000000 cycles"));
    assert!(warning.contains("maximum_automatic_exposure=120000000000000 cycles"));
    assert!(
        resolved.fleet_subnet_roots[0]
            .placement_cost
            .warning
            .is_some()
    );

    let application_subnet = subnet_text(7);
    let mut stale_ack = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet.clone(),
        acknowledge_fiduciary_cost: true,
    });
    stale_ack.fleet_subnet_roots[0].placement_subnet = application_subnet.clone();
    let application_catalog = catalog(vec![info(
        &application_subnet,
        SubnetKind::Application,
        SubnetSpecialization::None,
        "application",
    )]);
    assert!(matches!(
        resolve_document(&stale_ack, BuildNetwork::Ic, Some(&application_catalog)),
        Err(FleetInstallInputError::UnexpectedFiduciaryCostAcknowledgement { .. })
    ));
}

#[test]
fn profile_baselines_scale_rationally_and_round_up_to_ten_tcycles() {
    let application_subnet = subnet_text(7);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet.clone(),
        acknowledge_fiduciary_cost: false,
    });
    input.fleet_subnet_roots[0].icp_refill = None;
    let mut large = info(
        &application_subnet,
        SubnetKind::Application,
        SubnetSpecialization::None,
        "application",
    );
    large.node_count = Some(34);
    let large_catalog = catalog(vec![large]);

    assert!(matches!(
        resolve_document(&input, BuildNetwork::Ic, Some(&large_catalog)),
        Err(FleetInstallInputError::FundingProfileMinimum { .. })
    ));

    input.coordinator.creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(270 * TRILLION_CYCLES),
    };
    let coordinator = input
        .coordinator
        .root_funding
        .as_mut()
        .expect("Coordinator policy");
    coordinator.minimum_reserve_cycles = Cycles::new(80 * TRILLION_CYCLES);
    coordinator.maximum_cycles = Cycles::new(80 * TRILLION_CYCLES);
    coordinator.maximum_automatic_cycles = Cycles::new(320 * TRILLION_CYCLES);
    let root = &mut input.fleet_subnet_roots[0];
    root.root_creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(80 * TRILLION_CYCLES),
    };
    root.wasm_store_creation_funding = CreationFundingDocument::Cycles {
        cycles: Cycles::new(30 * TRILLION_CYCLES),
    };
    let root_policy = root.root_funding.as_mut().expect("Root policy");
    root_policy.request_threshold = Cycles::new(30 * TRILLION_CYCLES);
    root_policy.target_balance = Cycles::new(80 * TRILLION_CYCLES);
    root_policy.maximum_cycles = Cycles::new(80 * TRILLION_CYCLES);
    root_policy.maximum_automatic_cycles = Cycles::new(320 * TRILLION_CYCLES);

    let resolved = resolve_document(&input, BuildNetwork::Ic, Some(&large_catalog))
        .expect("34-node materialized values pass");
    assert_eq!(resolved.coordinator.placement_cost.node_count, 34);
    assert_eq!(
        resolved
            .coordinator
            .placement_cost
            .cost_multiplier_numerator,
        34
    );
    assert_eq!(
        resolved
            .coordinator
            .placement_cost
            .cost_multiplier_denominator,
        13
    );
}

fn document(selector: CoordinatorSubnetSelector) -> FleetInstallInputDocument {
    let application_subnet = subnet_text(7);
    FleetInstallInputDocument {
        schema_version: 1,
        funding_profile: FleetFundingProfile::SingleSubnet,
        operator: OperatorFundingDocument {
            principal: subnet_text(9),
            funding_account: "test-operator".to_string(),
            source: "test_fixture".to_string(),
            observed_at_unix_secs: FIXTURE_NOW_UNIX_SECS,
            valid_until_unix_secs: FIXTURE_NOW_UNIX_SECS + 300,
            balance: CreationFundingDocument::Cycles {
                cycles: Cycles::new(5_000_000_000_000_000),
            },
        },
        coordinator: CoordinatorInputDocument {
            subnet: selector,
            creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(100_000_000_000_000),
            },
            root_funding: Some(CoordinatorRootFundingPolicyDocument {
                minimum_reserve_cycles: Cycles::new(30_000_000_000_000),
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
            }),
        },
        fleet_subnet_roots: vec![FleetSubnetRootInputDocument {
            placement_subnet: application_subnet,
            acknowledge_fiduciary_cost: false,
            component_group_placements: BTreeMap::new(),
            component_admissions: BTreeMap::from([(
                "users".parse().expect("valid Component Spec ID"),
                8,
            )]),
            limits: FleetSubnetRootLimitsDocument {
                maximum_component_instances: 8,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 40_000_000,
                maximum_group_placements: 16,
                cycles_funding: CyclesFundingBudgetDocument {
                    window_secs: 3_600,
                    maximum_cycles: Cycles::new(10_000_000_000_000),
                },
            },
            canister_pool: CanisterPoolInputDocument {
                minimum_size: 3,
                maximum_size: 10,
                canister_cycles: Cycles::new(5_000_000_000_000),
                imports: Vec::new(),
            },
            root_funding: Some(FleetSubnetRootFundingPolicyDocument {
                request_threshold: Cycles::new(10_000_000_000_000),
                target_balance: Cycles::new(30_000_000_000_000),
                cooldown_secs: 30 * 24 * 60 * 60,
                window_secs: 90 * 24 * 60 * 60,
                maximum_cycles: Cycles::new(30_000_000_000_000),
                maximum_automatic_grants: 4,
                maximum_automatic_cycles: Cycles::new(120_000_000_000_000),
            }),
            icp_refill: Some(FleetSubnetRootIcpRefillPolicyDocument {
                max_refill_e8s_per_call: 100_000_000,
                window_secs: 86_400,
                maximum_refill_e8s: 200_000_000,
                minimum_icp_balance_e8s: 10_000_000,
                min_xdr_permyriad_per_icp: Some(40_000),
                ledger_canister_id: None,
                cmc_canister_id: None,
                allow_ic_system_canister_overrides: false,
                automatic: Some(FleetSubnetRootAutomaticIcpRefillPolicyDocument {
                    emergency_threshold: Cycles::new(5_000_000_000_000),
                    target_balance: Cycles::new(20_000_000_000_000),
                    maximum_automatic_refills: 4,
                    maximum_automatic_refill_e8s: 400_000_000,
                }),
            }),
            root_creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(30_000_000_000_000),
            },
            wasm_store_creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(10_000_000_000_000),
            },
        }],
    }
}

fn assert_invalid_policy(document: &FleetInstallInputDocument, expected_field_suffix: &str) {
    match resolve_document(document, BuildNetwork::Local, None) {
        Err(FleetInstallInputError::InvalidFundingPolicy { field, .. }) => {
            assert!(
                field.ends_with(expected_field_suffix),
                "unexpected invalid policy field: {field}"
            );
        }
        result => panic!("expected invalid protected funding policy, got {result:?}"),
    }
}

fn input_toml() -> String {
    let application_subnet = subnet_text(7);
    format!(
        r#"schema_version = 1
funding_profile = "single_subnet"

[operator]
principal = "{}"
funding_account = "test-operator"
source = "test_fixture"
observed_at_unix_secs = {FIXTURE_NOW_UNIX_SECS}
valid_until_unix_secs = 4102444800

[operator.balance]
kind = "cycles"
cycles = "5000T"

[coordinator.subnet]
kind = "explicit"
subnet = "{application_subnet}"
acknowledge_fiduciary_cost = false

[coordinator.creation_funding]
kind = "cycles"
cycles = "100T"

[coordinator.root_funding]
minimum_reserve_cycles = "30T"
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 4
maximum_automatic_cycles = "120T"

[[fleet_subnet_roots]]
placement_subnet = "{application_subnet}"
acknowledge_fiduciary_cost = false

[fleet_subnet_roots.component_admissions]
users = 8

[fleet_subnet_roots.canister_pool]
minimum_size = 3
maximum_size = 10
canister_cycles = "5T"
imports = []

[fleet_subnet_roots.root_funding]
request_threshold = "10T"
target_balance = "30T"
cooldown_secs = 2592000
window_secs = 7776000
maximum_cycles = "30T"
maximum_automatic_grants = 4
maximum_automatic_cycles = "120T"

[fleet_subnet_roots.icp_refill]
max_refill_e8s_per_call = 100000000
window_secs = 86400
maximum_refill_e8s = 200000000
minimum_icp_balance_e8s = 10000000
min_xdr_permyriad_per_icp = 40000

[fleet_subnet_roots.icp_refill.automatic]
emergency_threshold = "5T"
target_balance = "20T"
maximum_automatic_refills = 4
maximum_automatic_refill_e8s = 400000000

[fleet_subnet_roots.limits]
maximum_component_instances = 8
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000
maximum_group_placements = 16

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "10T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "30T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "10T"
"#,
        subnet_text(9)
    )
}

fn subnet(text: &str) -> SubnetId {
    SubnetId::from_principal(Principal::from_text(text).expect("valid Subnet principal"))
}

fn subnet_text(byte: u8) -> String {
    Principal::from_slice(&[byte; 29]).to_text()
}

fn catalog(subnets: Vec<SubnetInfo>) -> ValidatedSubnetCatalog {
    let subnet_principal = subnets
        .first()
        .expect("fixture Subnet")
        .subnet_principal
        .clone();
    catalog_with_ranges(
        subnets,
        vec![RoutingRange {
            start_canister_id: FIXTURE_CANISTER.to_string(),
            end_canister_id: FIXTURE_CANISTER.to_string(),
            subnet_principal,
        }],
    )
}

fn catalog_with_ranges(
    subnets: Vec<SubnetInfo>,
    routing_ranges: Vec<RoutingRange>,
) -> ValidatedSubnetCatalog {
    let registry_records = fixture_registry_records(&subnets);
    let raw = RawSubnetCatalog::new_mainnet_uncertified(
        UncertifiedCatalogCollection::new(
            1,
            DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
            FIXTURE_FETCHED_AT,
            "canic-test",
            "0.29.3",
            registry_records
                .len()
                .try_into()
                .expect("fixture Registry query count fits u64"),
        )
        .with_registry_evidence(
            SubnetCatalogRoutingSource::LegacyRoutingTable,
            registry_records,
        ),
        subnets,
        routing_ranges,
    )
    .expect("build raw fixture catalog");
    let validation = CatalogValidationContext::new(
        MAINNET_NETWORK,
        MAINNET_REGISTRY_CANISTER_ID,
        FIXTURE_NOW_UNIX_SECS,
        DEFAULT_CATALOG_MAX_FUTURE_SKEW_SECONDS,
    );
    ValidatedSubnetCatalog::try_from_raw(raw, &validation)
        .expect("validate fixture catalog authority")
}

fn fixture_registry_records(subnets: &[SubnetInfo]) -> Vec<SubnetCatalogRegistryRecordEvidence> {
    let evidence = |record| {
        SubnetCatalogRegistryRecordEvidence::uncertified_query(
            record,
            1,
            1,
            42,
            DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
            SubnetCatalogRegistryValueEncoding::Inline,
        )
    };
    let mut records = vec![
        evidence(SubnetCatalogRegistryRecordSubject::subnet_list()),
        evidence(SubnetCatalogRegistryRecordSubject::legacy_routing_table()),
    ];
    records.extend(subnets.iter().map(|subnet| {
        let principal = Principal::from_text(&subnet.subnet_principal)
            .expect("fixture Subnet principal must be valid");
        evidence(SubnetCatalogRegistryRecordSubject::subnet_record(principal))
    }));
    records
}

fn info(
    subnet_principal: &str,
    subnet_kind: SubnetKind,
    subnet_specialization: SubnetSpecialization,
    subnet_label: &str,
) -> SubnetInfo {
    let registry_subnet_type = match subnet_kind {
        SubnetKind::Unknown => 0,
        SubnetKind::Application => 1,
        SubnetKind::System => 2,
        SubnetKind::CloudEngine => 5,
    };
    SubnetInfo {
        subnet_principal: subnet_principal.to_string(),
        registry_subnet_type,
        subnet_kind,
        subnet_kind_source: ClassificationSource::Registry,
        subnet_specialization,
        subnet_specialization_source: ClassificationSource::Curated,
        geographic_scope: GeographicScope::Global,
        geographic_scope_source: ClassificationSource::Curated,
        subnet_label: subnet_label.to_string(),
        subnet_label_source: ClassificationSource::Curated,
        node_count: Some(13),
        charges_apply_by_default: subnet_kind.charges_apply_by_default(),
    }
}
