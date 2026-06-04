use crate::{
    CATALOG_SCHEMA_VERSION, CatalogError, ClassificationSource, GeographicScope, MAINNET_NETWORK,
    MAINNET_REGISTRY_CANISTER_ID, ResolveAs, ResolvedSubnetSubject, RoutingRange, SubnetCatalog,
    SubnetInfo, SubnetKind, SubnetSpecialization, parse_catalog_json, principal_bytes,
};

const SUBNET_A: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
const SUBNET_B: &str = "aaaaa-aa";

#[test]
fn catalog_schema_round_trips_through_json() {
    let catalog = fixture_catalog();
    let json = serde_json::to_string_pretty(&catalog).expect("serialize catalog");
    let decoded = parse_catalog_json(&json).expect("parse catalog");

    assert_eq!(decoded, catalog);
}

#[test]
fn unknown_future_schema_version_is_rejected() {
    let mut catalog = fixture_catalog();
    catalog.catalog_schema_version = CATALOG_SCHEMA_VERSION + 1;
    let json = serde_json::to_string(&catalog).expect("serialize catalog");

    let err = parse_catalog_json(&json).expect_err("future schema must fail");

    std::assert_matches!(
        err,
        CatalogError::UnsupportedSchemaVersion {
            found,
            supported: CATALOG_SCHEMA_VERSION
        } if found == CATALOG_SCHEMA_VERSION + 1
    );
}

#[test]
fn empty_subnets_and_empty_ranges_are_rejected() {
    let mut empty_subnets = fixture_catalog();
    empty_subnets.subnets.clear();
    std::assert_matches!(empty_subnets.validate(), Err(CatalogError::EmptySubnets));

    let mut empty_ranges = fixture_catalog();
    empty_ranges.routing_ranges.clear();
    std::assert_matches!(
        empty_ranges.validate(),
        Err(CatalogError::EmptyRoutingRanges)
    );
}

#[test]
fn resolver_maps_canister_by_inclusive_principal_byte_range() {
    let ids = sorted_principals([
        "ryjl3-tyaaa-aaaaa-aaaba-cai",
        "rrkah-fqaaa-aaaaa-aaaaq-cai",
        "r7inp-6aaaa-aaaaa-aaabq-cai",
        "t63gs-up777-77776-aaaba-cai",
        "uxrrr-q7777-77774-qaaaq-cai",
    ]);
    let mut catalog = fixture_catalog();
    catalog.routing_ranges = vec![RoutingRange {
        start_canister_id: ids[1].clone(),
        end_canister_id: ids[3].clone(),
        subnet_principal: SUBNET_A.to_string(),
    }];
    catalog.validate().expect("valid range");

    let start = catalog
        .resolve_principal(&ids[1], None)
        .expect("start boundary resolves");
    let inside = catalog
        .resolve_principal(&ids[2], None)
        .expect("inside resolves");
    let end = catalog
        .resolve_principal(&ids[3], None)
        .expect("end boundary resolves");

    assert_eq!(start.subnet.subnet_principal, SUBNET_A);
    assert_eq!(inside.subnet.subnet_principal, SUBNET_A);
    assert_eq!(end.subnet.subnet_principal, SUBNET_A);
    std::assert_matches!(
        catalog.resolve_principal(&ids[0], None),
        Err(CatalogError::RouteNotFound { .. })
    );
    std::assert_matches!(
        catalog.resolve_principal(&ids[4], None),
        Err(CatalogError::RouteNotFound { .. })
    );
}

#[test]
fn known_subnet_principal_wins_over_canister_range_interpretation() {
    let mut catalog = fixture_catalog();
    catalog.routing_ranges = vec![RoutingRange {
        start_canister_id: SUBNET_A.to_string(),
        end_canister_id: SUBNET_A.to_string(),
        subnet_principal: SUBNET_B.to_string(),
    }];
    catalog.validate().expect("valid synthetic overlap");

    let resolved = catalog
        .resolve_principal(SUBNET_A, None)
        .expect("known subnet wins");

    assert_eq!(resolved.resolved_as, ResolvedSubnetSubject::Subnet);
    assert_eq!(resolved.subnet.subnet_principal, SUBNET_A);
}

#[test]
fn forced_interpretations_are_explicit() {
    let catalog = fixture_catalog();

    let subnet = catalog
        .resolve_principal(SUBNET_A, Some(ResolveAs::Subnet))
        .expect("forced subnet");
    assert_eq!(subnet.resolved_as, ResolvedSubnetSubject::Subnet);

    let err = catalog
        .resolve_principal(SUBNET_A, Some(ResolveAs::Canister))
        .expect_err("subnet principal is not covered as a canister");
    std::assert_matches!(err, CatalogError::RouteNotFound { .. });
}

#[test]
fn resolver_accepts_unique_subnet_principal_prefix() {
    let catalog = fixture_catalog();

    let resolved = catalog
        .resolve_principal_or_prefix("rwl", None)
        .expect("subnet prefix resolves");

    assert_eq!(resolved.resolved_as, ResolvedSubnetSubject::Subnet);
    assert_eq!(resolved.resolved_from, "subnet_principal_prefix");
    assert_eq!(resolved.input_principal, "rwl");
    assert_eq!(resolved.subnet.subnet_principal, SUBNET_A);
}

#[test]
fn resolver_rejects_canister_boundary_prefix() {
    let catalog = fixture_catalog();

    let err = catalog
        .resolve_principal_or_prefix("ryj", None)
        .expect_err("partial canister principals are not accepted");

    std::assert_matches!(
        err,
        CatalogError::PrincipalPrefixNotFound { prefix } if prefix == "ryj"
    );
}

#[test]
fn resolver_rejects_ambiguous_principal_prefix() {
    let mut catalog = fixture_catalog();
    catalog.subnets.push(SubnetInfo {
        subnet_principal: "r7inp-6aaaa-aaaaa-aaabq-cai".to_string(),
        subnet_kind: SubnetKind::Application,
        subnet_kind_source: ClassificationSource::Registry,
        subnet_specialization: SubnetSpecialization::None,
        subnet_specialization_source: ClassificationSource::Computed,
        geographic_scope: GeographicScope::Global,
        geographic_scope_source: ClassificationSource::Computed,
        subnet_label: "application".to_string(),
        subnet_label_source: ClassificationSource::Computed,
        node_count: Some(13),
        charges_apply_by_default: true,
    });
    catalog.validate().expect("valid ambiguous fixture");

    let err = catalog
        .resolve_principal_or_prefix("r", None)
        .expect_err("ambiguous prefix fails");

    std::assert_matches!(
        err,
        CatalogError::AmbiguousPrincipalPrefix { prefix, matches }
            if prefix == "r" && matches.len() > 1
    );
}

#[test]
fn validation_rejects_unknown_routing_subnet_and_reversed_range() {
    let mut unknown = fixture_catalog();
    unknown.routing_ranges[0].subnet_principal = "uxrrr-q7777-77774-qaaaq-cai".to_string();
    std::assert_matches!(
        unknown.validate(),
        Err(CatalogError::UnknownRoutingSubnet { .. })
    );

    let ids = sorted_principals(["ryjl3-tyaaa-aaaaa-aaaba-cai", "rrkah-fqaaa-aaaaa-aaaaq-cai"]);
    let mut reversed = fixture_catalog();
    reversed.routing_ranges = vec![RoutingRange {
        start_canister_id: ids[1].clone(),
        end_canister_id: ids[0].clone(),
        subnet_principal: SUBNET_A.to_string(),
    }];
    std::assert_matches!(
        reversed.validate(),
        Err(CatalogError::InvalidRoutingRange { .. })
    );
}

fn fixture_catalog() -> SubnetCatalog {
    SubnetCatalog {
        catalog_schema_version: CATALOG_SCHEMA_VERSION,
        network: MAINNET_NETWORK.to_string(),
        registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
        registry_version: 123_456,
        fetched_at: "2026-06-04T00:00:00Z".to_string(),
        fetched_by: "fixture".to_string(),
        source_endpoint: "https://icp-api.io".to_string(),
        resolver_backend: "local-nns-subnet-catalog".to_string(),
        subnets: vec![
            SubnetInfo {
                subnet_principal: SUBNET_A.to_string(),
                subnet_kind: SubnetKind::Application,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::Fiduciary,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "fiduciary".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(34),
                charges_apply_by_default: true,
            },
            SubnetInfo {
                subnet_principal: SUBNET_B.to_string(),
                subnet_kind: SubnetKind::Application,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::European,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Europe,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "european".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(13),
                charges_apply_by_default: true,
            },
        ],
        routing_ranges: vec![RoutingRange {
            start_canister_id: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
            end_canister_id: "ryjl3-tyaaa-aaaaa-aaaba-cai".to_string(),
            subnet_principal: SUBNET_A.to_string(),
        }],
    }
}

fn sorted_principals<const N: usize>(ids: [&str; N]) -> Vec<String> {
    let mut ids = ids.map(str::to_string).to_vec();
    ids.sort_by(|left, right| {
        principal_bytes(left, "test")
            .expect("valid left")
            .cmp(&principal_bytes(right, "test").expect("valid right"))
    });
    ids
}
