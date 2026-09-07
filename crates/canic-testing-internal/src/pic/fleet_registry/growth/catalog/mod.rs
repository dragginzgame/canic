//! Module: pic::fleet_registry::growth::catalog
//!
//! Responsibility: retain synthetic placement evidence for the real PocketIC estate.
//! Boundary: this catalog is test input, never mainnet discovery or release evidence.

use canic_host::fleet_ensure::model::DesiredFleet;
use ic_query::subnet_catalog::{
    CATALOG_SCHEMA_VERSION, CLASSIFICATION_SCHEMA_VERSION, CatalogAssurance, ClassificationSource,
    GeographicScope, MAINNET_NETWORK, MAINNET_REGISTRY_CANISTER_ID, RESOLVER_SCHEMA_VERSION,
    RawSubnetCatalog, RoutingRange, SubnetCatalogProvenance, SubnetCatalogRegistryRecordEvidence,
    SubnetCatalogRegistryRecordSubject, SubnetCatalogRegistryValueEncoding,
    SubnetCatalogRoutingSource, SubnetInfo, SubnetKind, SubnetSpecialization, subnet_catalog_path,
};
use std::{fs, os::unix::fs::PermissionsExt as _, path::Path, process::Command, time::SystemTime};

pub(super) fn retain(workspace: &Path, desired: &DesiredFleet) {
    let subnet = desired.bootstrap.as_ref().unwrap().roots[0]
        .placement_subnet
        .into_principal();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let timestamp = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .unwrap();
    assert!(timestamp.status.success());
    let endpoint = "https://icp-api.io";
    let evidence = |subject| {
        SubnetCatalogRegistryRecordEvidence::uncertified_query(
            subject,
            1,
            1,
            u64::try_from(now.as_nanos()).unwrap(),
            endpoint,
            SubnetCatalogRegistryValueEncoding::Inline,
        )
    };
    let mut catalog = RawSubnetCatalog {
        catalog_schema_version: CATALOG_SCHEMA_VERSION,
        provenance: SubnetCatalogProvenance {
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 1,
            assurance: CatalogAssurance::UncertifiedQuery,
            source_endpoints: vec![endpoint.to_string()],
            agreement_digest: None,
            registry_query_call_count: 3,
            routing_source: SubnetCatalogRoutingSource::LegacyRoutingTable,
            registry_records: vec![
                evidence(SubnetCatalogRegistryRecordSubject::subnet_list()),
                evidence(SubnetCatalogRegistryRecordSubject::legacy_routing_table()),
                evidence(SubnetCatalogRegistryRecordSubject::subnet_record(subnet)),
            ],
            fetched_at: String::from_utf8(timestamp.stdout)
                .unwrap()
                .trim()
                .to_string(),
            certified_registry: None,
            fetched_by: "canic-pocketic-growth-fixture".to_string(),
            collector_version: env!("CARGO_PKG_VERSION").to_string(),
            classification_schema_version: CLASSIFICATION_SCHEMA_VERSION,
            classification_policy_digest: "00".repeat(32),
            resolver_schema_version: RESOLVER_SCHEMA_VERSION,
            resolver_backend: "local-nns-subnet-catalog".to_string(),
        },
        catalog_digest: "00".repeat(32),
        subnets: vec![SubnetInfo {
            subnet_principal: subnet.to_text(),
            registry_subnet_type: 1,
            subnet_kind: SubnetKind::Application,
            subnet_kind_source: ClassificationSource::Registry,
            subnet_specialization: SubnetSpecialization::None,
            subnet_specialization_source: ClassificationSource::Curated,
            geographic_scope: GeographicScope::Global,
            geographic_scope_source: ClassificationSource::Curated,
            subnet_label: "synthetic-growth".to_string(),
            subnet_label_source: ClassificationSource::Curated,
            node_count: Some(13),
            charges_apply_by_default: true,
        }],
        routing_ranges: desired
            .canisters
            .iter()
            .map(|canister| RoutingRange {
                start_canister_id: canister.principal.clone().unwrap(),
                end_canister_id: canister.principal.clone().unwrap(),
                subnet_principal: subnet.to_text(),
            })
            .collect(),
    };
    catalog
        .canonicalize_and_seal()
        .expect("seal synthetic placement evidence");
    let cache = canic_host::subnet_catalog::mainnet_subnet_catalog_cache_root(workspace);
    let path = subnet_catalog_path(&cache, MAINNET_NETWORK);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut directory = cache.clone();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    for component in path
        .parent()
        .unwrap()
        .strip_prefix(&cache)
        .unwrap()
        .components()
    {
        directory.push(component);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    }
    fs::write(&path, serde_json::to_vec(&catalog).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    canic_host::subnet_catalog::load_cached_mainnet_subnet_catalog(workspace, now.as_secs())
        .expect("validate the synthetic cache before any generator refresh can occur");
}
