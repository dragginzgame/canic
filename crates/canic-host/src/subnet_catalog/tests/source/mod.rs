//! Synthetic endpoint evidence for catalog policy tests; never mainnet observations.

use candid::Principal;
use ic_query::{
    nns::NnsSourceRequest,
    subnet_catalog::{
        CatalogError, ClassificationSource, GeographicScope, RawSubnetCatalog, RoutingRange,
        SubnetCatalogHostError, SubnetCatalogRegistryRecordEvidence,
        SubnetCatalogRegistryRecordSubject, SubnetCatalogRegistryValueEncoding,
        SubnetCatalogRoutingSource, SubnetCatalogSource, SubnetCatalogSourceFuture, SubnetInfo,
        SubnetKind, SubnetSpecialization, UncertifiedCatalogCollection,
    },
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy)]
pub(super) enum Reply {
    Matching,
    DifferentVersion,
    DifferentPayload,
    Unavailable,
}

pub(super) struct Source {
    pub reply: Reply,
    pub calls: AtomicUsize,
    pub version: u64,
}

impl Source {
    pub const fn new(reply: Reply, version: u64) -> Self {
        Self {
            reply,
            calls: AtomicUsize::new(0),
            version,
        }
    }
}

impl SubnetCatalogSource for Source {
    fn fetch_catalog<'a>(&'a self, request: &'a NnsSourceRequest) -> SubnetCatalogSourceFuture<'a> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Box::pin(std::future::ready(catalog(self, request)))
    }
}

fn catalog(
    source: &Source,
    request: &NnsSourceRequest,
) -> Result<RawSubnetCatalog, SubnetCatalogHostError> {
    let different = request.endpoint == crate::subnet_catalog::MAINNET_CATALOG_ENDPOINTS[1];
    if different && matches!(source.reply, Reply::Unavailable) {
        return Err(SubnetCatalogHostError::Catalog(CatalogError::EmptySubnets));
    }
    let version =
        source.version + u64::from(different && matches!(source.reply, Reply::DifferentVersion));
    let subnet = Principal::from_slice(&[42]);
    let canister = Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap();
    let evidence = |subject| {
        SubnetCatalogRegistryRecordEvidence::uncertified_query(
            subject,
            version,
            version,
            1_780_531_200_000_000_000,
            &request.endpoint,
            SubnetCatalogRegistryValueEncoding::Inline,
        )
    };
    let collection = UncertifiedCatalogCollection::new(
        version,
        &request.endpoint,
        &request.fetched_at,
        &request.fetched_by,
        "fixture",
        3,
    )
    .with_registry_evidence(
        SubnetCatalogRoutingSource::CanisterRanges,
        vec![
            evidence(SubnetCatalogRegistryRecordSubject::subnet_list()),
            evidence(SubnetCatalogRegistryRecordSubject::canister_ranges(
                canister,
            )),
            evidence(SubnetCatalogRegistryRecordSubject::subnet_record(subnet)),
        ],
    );
    Ok(RawSubnetCatalog::new_mainnet_uncertified(
        collection,
        vec![SubnetInfo {
            subnet_principal: subnet.to_text(),
            registry_subnet_type: 1,
            subnet_kind: SubnetKind::Application,
            subnet_kind_source: ClassificationSource::Registry,
            subnet_specialization: SubnetSpecialization::None,
            subnet_specialization_source: ClassificationSource::Curated,
            geographic_scope: GeographicScope::Global,
            geographic_scope_source: ClassificationSource::Curated,
            subnet_label: "synthetic".into(),
            subnet_label_source: ClassificationSource::Curated,
            node_count: Some(
                if different && matches!(source.reply, Reply::DifferentPayload) {
                    34
                } else {
                    13
                },
            ),
            charges_apply_by_default: true,
        }],
        vec![RoutingRange {
            start_canister_id: canister.to_text(),
            end_canister_id: canister.to_text(),
            subnet_principal: subnet.to_text(),
        }],
    )?)
}
