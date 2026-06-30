use super::super::*;
use super::{finding, refresh_resume_safety};
use ic_query::subnet_catalog::{
    DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT, ResolveAs, SubnetCatalogCacheRequest,
    load_or_refresh_subnet_catalog,
};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAINNET_NETWORK: &str = "ic";
const CLOUD_ENGINE_SUBNET_KIND: &str = "cloud_engine";
pub(in crate::deployment_truth) const ROOT_AUTH_SUBNET_EVIDENCE_MISSING_CODE: &str =
    "root_auth_subnet_evidence_missing";
pub(in crate::deployment_truth) const ROOT_AUTH_CLOUD_ENGINE_SUBNET_CODE: &str =
    "root_auth_cloud_engine_subnet";

pub(in crate::deployment_truth) fn apply_root_auth_signer_subnet_check(
    diff: &mut DeploymentDiffV1,
    inventory: &DeploymentInventoryV1,
    network: &str,
    icp_root: &Path,
) {
    apply_root_auth_signer_subnet_check_with_source(
        diff,
        inventory,
        network,
        icp_root,
        &LiveSubnetCatalogRootSubnetEvidenceSource,
    );
}

pub(in crate::deployment_truth) fn apply_root_auth_signer_subnet_check_with_source(
    diff: &mut DeploymentDiffV1,
    inventory: &DeploymentInventoryV1,
    network: &str,
    icp_root: &Path,
    source: &dyn RootSubnetEvidenceSource,
) {
    if network != MAINNET_NETWORK {
        return;
    }
    let Some(root) = &inventory.observed_root else {
        return;
    };
    let evidence = match source.root_subnet_evidence(network, icp_root, &root.observed_canister_id)
    {
        Ok(evidence) => evidence,
        Err(err) => {
            diff.hard_failures.push(finding(
                ROOT_AUTH_SUBNET_EVIDENCE_MISSING_CODE,
                format!(
                    "cannot verify root-auth signer subnet kind for {} with the NNS subnet catalog: {err}",
                    root.observed_canister_id
                ),
                SafetySeverityV1::HardFailure,
                Some(root.observed_canister_id.clone()),
            ));
            refresh_resume_safety(diff);
            return;
        }
    };
    if evidence.subnet_kind == CLOUD_ENGINE_SUBNET_KIND {
        diff.hard_failures.push(finding(
            ROOT_AUTH_CLOUD_ENGINE_SUBNET_CODE,
            format!(
                "root canister {} resolves to cloud_engine subnet {}; Canic root-auth policy does not allow root signing canisters on cloud_engine subnets",
                root.observed_canister_id, evidence.subnet_principal
            ),
            SafetySeverityV1::HardFailure,
            Some(root.observed_canister_id.clone()),
        ));
        refresh_resume_safety(diff);
    }
}

///
/// RootSubnetEvidence
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::deployment_truth) struct RootSubnetEvidence {
    pub subnet_principal: String,
    pub subnet_kind: String,
}

pub(in crate::deployment_truth) trait RootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        network: &str,
        icp_root: &Path,
        canister_id: &str,
    ) -> Result<RootSubnetEvidence, String>;
}

///
/// LiveSubnetCatalogRootSubnetEvidenceSource
///
struct LiveSubnetCatalogRootSubnetEvidenceSource;

impl RootSubnetEvidenceSource for LiveSubnetCatalogRootSubnetEvidenceSource {
    fn root_subnet_evidence(
        &self,
        network: &str,
        icp_root: &Path,
        canister_id: &str,
    ) -> Result<RootSubnetEvidence, String> {
        let request = SubnetCatalogCacheRequest {
            icp_root: icp_root.to_path_buf(),
            network: network.to_string(),
        };
        let cached = load_or_refresh_subnet_catalog(
            &request,
            DEFAULT_SUBNET_CATALOG_SOURCE_ENDPOINT,
            now_unix_secs()?,
        )
        .map_err(|err| err.to_string())?;
        let resolved = cached
            .catalog
            .resolve_principal(canister_id, Some(ResolveAs::Canister))
            .map_err(|err| err.to_string())?;

        Ok(RootSubnetEvidence {
            subnet_principal: resolved.subnet.subnet_principal,
            subnet_kind: resolved.subnet.subnet_kind.as_str().to_string(),
        })
    }
}

fn now_unix_secs() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system clock is before unix epoch: {err}"))
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_query::subnet_catalog::{
        CATALOG_SCHEMA_VERSION, ClassificationSource, GeographicScope,
        MAINNET_REGISTRY_CANISTER_ID, RoutingRange, SubnetCatalog, SubnetInfo, SubnetKind,
        SubnetSpecialization, catalog_to_pretty_json, subnet_catalog_path,
    };
    use std::{fs, path::PathBuf};

    const SUBNET: &str = "rwlgt-iiaaa-aaaaa-aaaaa-cai";
    const CANISTER: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

    #[test]
    fn subnet_catalog_source_resolves_cached_canister_without_icq_process() {
        let root = temp_root("root-subnet-catalog-source");
        let path = subnet_catalog_path(&root, MAINNET_NETWORK);
        fs::create_dir_all(path.parent().expect("catalog has parent"))
            .expect("create catalog parent");
        fs::write(
            &path,
            catalog_to_pretty_json(&fixture_catalog()).expect("catalog serializes"),
        )
        .expect("write catalog");

        let evidence = LiveSubnetCatalogRootSubnetEvidenceSource
            .root_subnet_evidence(MAINNET_NETWORK, &root, CANISTER)
            .expect("resolve cached canister");

        let _ = fs::remove_dir_all(root);
        assert_eq!(evidence.subnet_principal, SUBNET);
        assert_eq!(evidence.subnet_kind, CLOUD_ENGINE_SUBNET_KIND);
    }

    fn temp_root(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("canic-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn fixture_catalog() -> SubnetCatalog {
        SubnetCatalog {
            catalog_schema_version: CATALOG_SCHEMA_VERSION,
            network: MAINNET_NETWORK.to_string(),
            registry_canister_id: MAINNET_REGISTRY_CANISTER_ID.to_string(),
            registry_version: 123_456,
            fetched_at: "2026-06-26T00:00:00Z".to_string(),
            fetched_by: "fixture".to_string(),
            source_endpoint: "https://icp-api.io".to_string(),
            resolver_backend: "fixture".to_string(),
            subnets: vec![SubnetInfo {
                subnet_principal: SUBNET.to_string(),
                subnet_kind: SubnetKind::CloudEngine,
                subnet_kind_source: ClassificationSource::Registry,
                subnet_specialization: SubnetSpecialization::Unknown,
                subnet_specialization_source: ClassificationSource::Unknown,
                geographic_scope: GeographicScope::Global,
                geographic_scope_source: ClassificationSource::Curated,
                subnet_label: "cloud-engine".to_string(),
                subnet_label_source: ClassificationSource::Curated,
                node_count: Some(13),
                charges_apply_by_default: true,
            }],
            routing_ranges: vec![RoutingRange {
                start_canister_id: CANISTER.to_string(),
                end_canister_id: CANISTER.to_string(),
                subnet_principal: SUBNET.to_string(),
            }],
        }
    }
}
