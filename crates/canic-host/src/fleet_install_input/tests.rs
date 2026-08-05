//! Module: fleet_install_input::tests
//!
//! Responsibility: qualify strict document decoding and trusted placement/funding resolution.
//! Does not own: immutable Fleet plan persistence or install workflow mutation ordering.

use super::*;
use crate::test_support::temp_dir;
use std::fs;

use ic_query::subnet_catalog::{
    CatalogValidationContext, ClassificationSource, DEFAULT_CATALOG_MAX_FUTURE_SKEW_SECONDS,
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, GeographicScope, MAINNET_NETWORK,
    MAINNET_REGISTRY_CANISTER_ID, RawSubnetCatalog, RoutingRange, SubnetInfo,
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
            cycles: 3_000_000_000_000
        }
    );
    let [root] = resolved.fleet_subnet_roots.as_slice() else {
        panic!("proof input must resolve exactly one Fleet Subnet Root");
    };
    assert_eq!(root.placement_subnet, subnet(FIDUCIARY_SUBNET));
    assert_eq!(root.component_admissions.len(), 2);
    assert_eq!(root.limits.maximum_component_instances, 2);
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
            cycles: 5_000_000_000_000
        }
    );
    assert_eq!(
        root.wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 3_000_000_000_000
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
            cycles: 20_000_000_000_000
        }
    );
    assert_eq!(
        root.wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 3_000_000_000_000
        }
    );
}

#[test]
fn local_document_resolves_exact_explicit_placement_and_cycles() {
    let application_subnet = subnet_text(7);
    let document = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet.clone(),
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
            cycles: 2_000_000_000_000
        }
    );
    assert_eq!(resolved.fleet_subnet_roots.len(), 1);
    assert_eq!(
        resolved.fleet_subnet_roots[0].root_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000
        }
    );
    assert_eq!(
        resolved.fleet_subnet_roots[0].wasm_store_creation_funding,
        PlannedCanisterCreationFunding::Cycles {
            cycles: 2_000_000_000_000
        }
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
fn public_recommended_and_profile_select_exact_unique_application_subnets() {
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
    let recommended = resolve_document(
        &document(CoordinatorSubnetSelector::Recommended),
        BuildNetwork::Ic,
        Some(&catalog),
    )
    .expect("resolve recommended");
    assert_eq!(
        recommended.coordinator.coordinator_subnet,
        subnet(FIDUCIARY_SUBNET)
    );

    let profile = resolve_document(
        &document(CoordinatorSubnetSelector::Profile {
            profile: "european".to_string(),
        }),
        BuildNetwork::Ic,
        Some(&catalog),
    )
    .expect("resolve profile");
    assert_eq!(
        profile.coordinator.coordinator_subnet,
        subnet(EUROPEAN_SUBNET)
    );
}

#[test]
fn public_resolution_enforces_trusted_eligibility_and_funding_method() {
    let system_subnet = subnet_text(8);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: system_subnet.clone(),
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
fn nonpublic_network_rejects_derived_selectors_and_icp_funding() {
    assert!(matches!(
        resolve_document(
            &document(CoordinatorSubnetSelector::Recommended),
            BuildNetwork::Local,
            None
        ),
        Err(FleetInstallInputError::TrustedMetadataRequired { .. })
    ));

    let application_subnet = subnet_text(7);
    let mut input = document(CoordinatorSubnetSelector::Explicit {
        subnet: application_subnet,
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
            cycles: Cycles::new(2_000_000_000_000)
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
            cycles: Cycles::new(2_000_000_000_000)
        }
    );
    assert_eq!(
        document.fleet_subnet_roots[0].wasm_store_creation_funding,
        CreationFundingDocument::Cycles {
            cycles: Cycles::new(2_000_000_000_000)
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

fn document(selector: CoordinatorSubnetSelector) -> FleetInstallInputDocument {
    let application_subnet = subnet_text(7);
    FleetInstallInputDocument {
        schema_version: 1,
        coordinator: CoordinatorInputDocument {
            subnet: selector,
            creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(2_000_000_000_000),
            },
        },
        fleet_subnet_roots: vec![FleetSubnetRootInputDocument {
            placement_subnet: application_subnet,
            component_admissions: BTreeMap::from([(
                "users".parse().expect("valid Component Spec ID"),
                8,
            )]),
            limits: FleetSubnetRootLimitsDocument {
                maximum_component_instances: 8,
                maximum_registry_bytes: 16_777_216,
                maximum_wasm_store_bytes: 40_000_000,
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
            root_creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(2_000_000_000_000),
            },
            wasm_store_creation_funding: CreationFundingDocument::Cycles {
                cycles: Cycles::new(2_000_000_000_000),
            },
        }],
    }
}

fn input_toml() -> String {
    let application_subnet = subnet_text(7);
    format!(
        r#"schema_version = 1

[coordinator.subnet]
kind = "explicit"
subnet = "{application_subnet}"

[coordinator.creation_funding]
kind = "cycles"
cycles = "2T"

[[fleet_subnet_roots]]
placement_subnet = "{application_subnet}"

[fleet_subnet_roots.component_admissions]
users = 8

[fleet_subnet_roots.canister_pool]
minimum_size = 3
maximum_size = 10
canister_cycles = "5T"
imports = []

[fleet_subnet_roots.limits]
maximum_component_instances = 8
maximum_registry_bytes = 16777216
maximum_wasm_store_bytes = 40000000

[fleet_subnet_roots.limits.cycles_funding]
window_secs = 3600
maximum_cycles = "10T"

[fleet_subnet_roots.root_creation_funding]
kind = "cycles"
cycles = "2T"

[fleet_subnet_roots.wasm_store_creation_funding]
kind = "cycles"
cycles = "2T"
"#
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
    let raw = RawSubnetCatalog::new_mainnet_uncertified(
        UncertifiedCatalogCollection::new(
            1,
            DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
            FIXTURE_FETCHED_AT,
            "canic-test",
            "0.29.3",
            1,
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
