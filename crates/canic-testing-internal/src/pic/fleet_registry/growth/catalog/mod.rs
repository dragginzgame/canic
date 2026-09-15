//! Module: pic::fleet_registry::growth::catalog
//!
//! Responsibility: retain synthetic placement evidence for the real PocketIC estate.
//! Boundary: this catalog is test input, never mainnet discovery or release evidence.

use candid::Principal;
use canic_host::{
    fleet_ensure::model::DesiredFleet,
    subnet_catalog::{MAINNET_CATALOG_ENDPOINTS, mainnet_subnet_catalog_cache_root},
};
use ic_query::{
    nns::NnsSourceRequest,
    subnet_catalog::{
        CatalogAssurance, CatalogSourceSelection, ClassificationSource, GeographicScope,
        MAINNET_NETWORK, RawSubnetCatalog, RoutingRange, SubnetCatalogCacheRequest,
        SubnetCatalogHostError, SubnetCatalogLoadRequest, SubnetCatalogRegistryRecordEvidence,
        SubnetCatalogRegistryRecordSubject, SubnetCatalogRegistryValueEncoding,
        SubnetCatalogRoutingSource, SubnetCatalogSource, SubnetCatalogSourceFuture, SubnetInfo,
        SubnetKind, SubnetSpecialization, UncertifiedCatalogCollection,
        load_subnet_catalog_detailed_with_source,
    },
};
use std::{path::Path, time::SystemTime};

pub(super) fn retain(workspace: &Path, desired: &DesiredFleet) {
    let subnet = desired.bootstrap.as_ref().unwrap().roots[0]
        .placement_subnet
        .into_principal();
    let ranges = desired
        .canisters
        .iter()
        .map(|canister| RoutingRange {
            start_canister_id: canister.principal.clone().unwrap(),
            end_canister_id: canister.principal.clone().unwrap(),
            subnet_principal: subnet.to_text(),
        })
        .collect();
    retain_catalog(workspace, SyntheticCatalog { subnet, ranges });
}

fn retain_catalog(workspace: &Path, source: SyntheticCatalog) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let request = SubnetCatalogLoadRequest::refresh_missing_or_invalid(
        SubnetCatalogCacheRequest::new(
            mainnet_subnet_catalog_cache_root(workspace),
            MAINNET_NETWORK,
        ),
        CatalogSourceSelection::multi_endpoint_agreement(
            MAINNET_CATALOG_ENDPOINTS
                .iter()
                .map(|endpoint| (*endpoint).to_string())
                .collect(),
        ),
        now,
    )
    .with_minimum_assurance(CatalogAssurance::MultiEndpointAgreement);
    load_subnet_catalog_detailed_with_source(&request, &source)
        .expect("validate both synthetic endpoint snapshots and publish the catalog");
    canic_host::subnet_catalog::load_cached_mainnet_subnet_catalog(workspace, now)
        .expect("validate synthetic placement before generator refresh can occur");
}

struct SyntheticCatalog {
    subnet: Principal,
    ranges: Vec<RoutingRange>,
}

impl SubnetCatalogSource for SyntheticCatalog {
    fn fetch_catalog<'a>(&'a self, request: &'a NnsSourceRequest) -> SubnetCatalogSourceFuture<'a> {
        Box::pin(std::future::ready(self.snapshot(request)))
    }
}

impl SyntheticCatalog {
    fn snapshot(
        &self,
        request: &NnsSourceRequest,
    ) -> Result<RawSubnetCatalog, SubnetCatalogHostError> {
        let start = self
            .ranges
            .iter()
            .map(|range| Principal::from_text(&range.start_canister_id).unwrap())
            .min()
            .unwrap();
        let evidence = |subject| {
            SubnetCatalogRegistryRecordEvidence::uncertified_query(
                subject,
                1,
                1,
                1,
                &request.endpoint,
                SubnetCatalogRegistryValueEncoding::Inline,
            )
        };
        let collection = UncertifiedCatalogCollection::new(
            1,
            &request.endpoint,
            &request.fetched_at,
            "canic-pocketic-growth-fixture",
            env!("CARGO_PKG_VERSION"),
            3,
        )
        .with_registry_evidence(
            SubnetCatalogRoutingSource::CanisterRanges,
            vec![
                evidence(SubnetCatalogRegistryRecordSubject::subnet_list()),
                evidence(SubnetCatalogRegistryRecordSubject::canister_ranges(start)),
                evidence(SubnetCatalogRegistryRecordSubject::subnet_record(
                    self.subnet,
                )),
            ],
        );
        Ok(RawSubnetCatalog::new_mainnet_uncertified(
            collection,
            vec![SubnetInfo {
                subnet_principal: self.subnet.to_text(),
                registry_subnet_type: 1,
                subnet_kind: SubnetKind::Application,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::None,
                subnet_specialization_source: ClassificationSource::Curated,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "synthetic-growth".into(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(13),
                charges_apply_by_default: true,
            }],
            self.ranges.clone(),
        )?)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    #[test]
    pub(in crate::pic::fleet_registry) fn synthetic_growth_catalog_meets_host_agreement_policy() {
        let scratch = std::env::temp_dir().join(format!(
            "canic-growth-catalog-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        let subnet = Principal::from_slice(&[42]);
        let canister = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap();
        retain_catalog(
            &scratch,
            SyntheticCatalog {
                subnet,
                ranges: vec![RoutingRange {
                    start_canister_id: canister.to_text(),
                    end_canister_id: canister.to_text(),
                    subnet_principal: subnet.to_text(),
                }],
            },
        );
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let catalog =
            canic_host::subnet_catalog::load_cached_mainnet_subnet_catalog(&scratch, now).unwrap();
        assert_eq!(
            catalog.catalog.provenance().assurance,
            CatalogAssurance::MultiEndpointAgreement
        );
        assert_eq!(
            catalog
                .catalog
                .resolve_canister_route(&canister.to_text())
                .unwrap()
                .subnet,
            subnet
        );
        std::fs::remove_dir_all(scratch).unwrap();
    }
}
