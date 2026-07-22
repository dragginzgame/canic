use super::super::model::DEFAULT_INITIAL_CYCLES;
use super::labels::{metrics_profile_label, metrics_profile_tiers_label};
use canic_core::bootstrap::compiled::ConfigModel;
use std::collections::{BTreeMap, BTreeSet};

// Enumerate verbose configured details from one validated snapshot.
pub(in crate::release_set) fn configured_role_details_from_config(
    config: &ConfigModel,
) -> BTreeMap<String, Vec<String>> {
    let mut details = BTreeMap::<String, BTreeSet<String>>::new();

    for role in &config.app_index {
        details
            .entry(role.as_str().to_string())
            .or_default()
            .insert("app_index".to_string());
    }

    for subnet in config.subnets.values() {
        for role in subnet.auto_create_roles() {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("auto_create".to_string());
        }
        for role in subnet.subnet_index_roles() {
            details
                .entry(role.as_str().to_string())
                .or_default()
                .insert("subnet_index".to_string());
        }

        for (role, canister) in &subnet.canisters {
            let role_details = details.entry(role.as_str().to_string()).or_default();
            let profile = canister.resolved_metrics_profile(role);
            let profile_source = if canister.metrics.profile.is_some() {
                "configured"
            } else {
                "inferred"
            };
            role_details.insert(format!(
                "metrics profile={} tiers={} ({profile_source})",
                metrics_profile_label(profile),
                metrics_profile_tiers_label(profile)
            ));
            if canister.initial_cycles.to_u128() != DEFAULT_INITIAL_CYCLES {
                role_details.insert(format!("initial_cycles={}", canister.initial_cycles));
            }
            if canister.auth.delegated_token_issuer {
                role_details.insert("auth delegated-token-issuer".to_string());
            }
            if canister.auth.delegated_token_verifier {
                role_details.insert("auth delegated-token-verifier".to_string());
            }
            if canister.auth.role_attestation_cache {
                role_details.insert("auth role-attestation-cache".to_string());
            }
            if canister.standards.icrc21 {
                role_details.insert("standard icrc21".to_string());
            }
            if let Some(scaling) = &canister.scaling {
                for (pool_name, pool) in &scaling.pools {
                    role_details.insert(format!(
                        "scaling {pool_name}->{} initial={} min={} max={}",
                        pool.canister_role.as_str(),
                        pool.policy.initial_workers,
                        pool.policy.min_workers,
                        pool.policy.max_workers
                    ));
                }
            }
            if let Some(sharding) = &canister.sharding {
                for (pool_name, pool) in &sharding.pools {
                    role_details.insert(format!(
                        "sharding {pool_name}->{} cap={} initial={} max={}",
                        pool.canister_role.as_str(),
                        pool.policy.capacity,
                        pool.policy.initial_shards,
                        pool.policy.max_shards
                    ));
                }
            }
            if let Some(directory) = &canister.directory {
                for (pool_name, pool) in &directory.pools {
                    role_details.insert(format!(
                        "directory {pool_name}->{} key={}",
                        pool.canister_role.as_str(),
                        pool.key_name
                    ));
                }
            }
        }
    }

    details
        .into_iter()
        .filter(|(_, details)| !details.is_empty())
        .map(|(role, details)| (role, details.into_iter().collect()))
        .collect()
}
