//! Module: fleet_install_plan::decision
//!
//! Responsibility: purely compile complete fresh-Fleet counts, funding, admission, and digest.
//! Does not own: evidence collection, rendering, persistence, builds, clocks, or IC effects.
//! Boundary: successful output contains every decision-bearing fact and no unresolved blocker.

use super::CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES;
use super::model::*;
use candid::Principal;
use canic_core::cdk::utils::hash::hex_bytes;
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const FRESH_FLEET_DEPLOYMENT_PLAN_SCHEMA_VERSION: u16 = 1;
const PLAN_DIGEST_DOMAIN: &[u8] = b"canic-deployment-plan:v1\0";

/// Compile one complete canonical fresh-Fleet decision without reading or mutating ambient state.
pub fn compile_fresh_fleet_deployment_plan(
    request: FreshFleetDeploymentPlanRequest,
) -> Result<FreshFleetDeploymentPlanV1, FreshFleetDeploymentPlanError> {
    let maximum_operator_debit = fresh_fleet_maximum_operator_debit(&request.preflight)?;
    compile_fresh_fleet_deployment_plan_with_operator_debit(request, &maximum_operator_debit)
}

/// Compile an exact retained-session decision against only the debit the host can still issue.
///
/// The returned canonical plan and digest continue to bind the original maximum operator debit.
pub fn compile_fresh_fleet_deployment_plan_with_operator_debit(
    request: FreshFleetDeploymentPlanRequest,
    required_operator_debit: &PlannedCanisterCreationFunding,
) -> Result<FreshFleetDeploymentPlanV1, FreshFleetDeploymentPlanError> {
    validate_authority(&request.preflight, &request.authority)?;
    let counts = compile_counts(&request.preflight)?;
    let funding_requirements = compile_funding_requirements(&request.preflight, counts)?;
    let maximum_operator_debit = maximum_operator_debit(&funding_requirements)?;
    validate_operator_balance(&request.authority.operator.balance, required_operator_debit)?;
    let plan_digest = plan_digest(
        &request.preflight,
        &request.authority,
        counts,
        &funding_requirements,
        &maximum_operator_debit,
    )?;

    Ok(FreshFleetDeploymentPlanV1 {
        schema_version: FRESH_FLEET_DEPLOYMENT_PLAN_SCHEMA_VERSION,
        preflight: request.preflight,
        authority: request.authority,
        counts,
        funding_requirements,
        maximum_operator_debit,
        operator_balance_sufficient: true,
        plan_digest,
    })
}

/// Compute the exact maximum debit borne by the installation operator.
pub fn fresh_fleet_maximum_operator_debit(
    preflight: &FreshFleetPreflightV1,
) -> Result<PlannedCanisterCreationFunding, FreshFleetDeploymentPlanError> {
    let counts = compile_counts(preflight)?;
    maximum_operator_debit(&compile_funding_requirements(preflight, counts)?)
}

fn validate_authority(
    preflight: &FreshFleetPreflightV1,
    authority: &FreshFleetDecisionAuthorityV1,
) -> Result<(), FreshFleetDeploymentPlanError> {
    for (field, value) in [
        ("app_config_sha256", authority.app_config_sha256.as_str()),
        ("fleet_input_sha256", authority.fleet_input_sha256.as_str()),
    ] {
        validate_sha256(field, value)?;
    }
    validate_nonempty("requested_environment", &authority.requested_environment)?;
    validate_operator(&authority.operator)?;
    validate_release_source(preflight, &authority.release_source)?;
    validate_catalog(&authority.catalog)
}

fn validate_release_source(
    preflight: &FreshFleetPreflightV1,
    source: &FreshFleetReleaseSourceV1,
) -> Result<(), FreshFleetDeploymentPlanError> {
    let artifacts = source.expected_artifacts();
    if artifacts.is_empty()
        || !artifacts.is_sorted()
        || artifacts.windows(2).any(|window| window[0] == window[1])
    {
        return Err(FreshFleetDeploymentPlanError::NonCanonicalArtifactInventory);
    }
    for artifact in artifacts {
        validate_nonempty("release_source.expected_artifacts.role", &artifact.role)?;
        validate_nonempty(
            "release_source.expected_artifacts.package",
            &artifact.package,
        )?;
    }

    match source {
        FreshFleetReleaseSourceV1::Workspace {
            builder_version,
            cargo_lock_sha256,
            source_snapshot_sha256,
            ..
        } => {
            if preflight.release_build_id.is_some() {
                return Err(FreshFleetDeploymentPlanError::ReleaseBuildIdentityMismatch);
            }
            validate_nonempty("release_source.builder_version", builder_version)?;
            validate_sha256("release_source.cargo_lock_sha256", cargo_lock_sha256)?;
            validate_sha256(
                "release_source.source_snapshot_sha256",
                source_snapshot_sha256,
            )
        }
        FreshFleetReleaseSourceV1::Finalized {
            release_build_id,
            builder_version,
            release_build_plan_sha256,
            release_set_manifest_sha256,
            ..
        } => {
            if preflight.release_build_id != Some(*release_build_id) {
                return Err(FreshFleetDeploymentPlanError::ReleaseBuildIdentityMismatch);
            }
            validate_nonempty("release_source.builder_version", builder_version)?;
            validate_sha256(
                "release_source.release_build_plan_sha256",
                release_build_plan_sha256,
            )?;
            validate_sha256(
                "release_source.release_set_manifest_sha256",
                release_set_manifest_sha256,
            )
        }
    }
}

fn validate_catalog(
    catalog: &FreshFleetCatalogEvidenceV1,
) -> Result<(), FreshFleetDeploymentPlanError> {
    match catalog {
        FreshFleetCatalogEvidenceV1::NotRequired { network } => {
            validate_nonempty("catalog.network", network)
        }
        FreshFleetCatalogEvidenceV1::Validated {
            network,
            assurance,
            source_endpoints,
            catalog_sha256,
            ..
        } => {
            validate_nonempty("catalog.network", network)?;
            validate_nonempty("catalog.assurance", assurance)?;
            if source_endpoints.is_empty()
                || !source_endpoints.is_sorted()
                || source_endpoints
                    .windows(2)
                    .any(|window| window[0] == window[1])
            {
                return Err(FreshFleetDeploymentPlanError::EmptyAuthority {
                    field: "catalog.source_endpoints",
                });
            }
            validate_sha256("catalog.catalog_sha256", catalog_sha256)
        }
    }
}

fn validate_operator(
    operator: &FreshFleetOperatorFundingEvidenceV1,
) -> Result<(), FreshFleetDeploymentPlanError> {
    let principal = Principal::from_text(&operator.principal).map_err(|_| {
        FreshFleetDeploymentPlanError::EmptyAuthority {
            field: "operator.principal",
        }
    })?;
    if principal == Principal::anonymous() {
        return Err(FreshFleetDeploymentPlanError::AnonymousOperator);
    }
    if principal.to_text() != operator.principal {
        return Err(FreshFleetDeploymentPlanError::EmptyAuthority {
            field: "operator.principal",
        });
    }
    validate_nonempty("operator.funding_account", &operator.funding_account)?;
    validate_nonempty("operator.source", &operator.source)?;
    if operator.observed_at_unix_secs == 0
        || operator.valid_until_unix_secs <= operator.observed_at_unix_secs
    {
        return Err(FreshFleetDeploymentPlanError::EmptyAuthority {
            field: "operator.balance_validity",
        });
    }
    if !operator.balance_fresh {
        return Err(FreshFleetDeploymentPlanError::StaleOperatorBalance);
    }
    Ok(())
}

const fn validate_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), FreshFleetDeploymentPlanError> {
    if value.is_empty() {
        Err(FreshFleetDeploymentPlanError::EmptyAuthority { field })
    } else {
        Ok(())
    }
}

fn validate_sha256(field: &'static str, value: &str) -> Result<(), FreshFleetDeploymentPlanError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'));
    if valid {
        Ok(())
    } else {
        Err(FreshFleetDeploymentPlanError::InvalidSha256 { field })
    }
}

fn compile_counts(
    preflight: &FreshFleetPreflightV1,
) -> Result<FreshFleetCanisterCountsV1, FreshFleetDeploymentPlanError> {
    let root_canisters = u32::try_from(preflight.fleet_subnet_roots.len()).map_err(|_| {
        FreshFleetDeploymentPlanError::CountOverflow {
            subject: "Fleet Subnet Root",
        }
    })?;
    let component_canisters = checked_root_count_sum(
        preflight,
        |root| root.initial_component_canisters,
        "initial Component",
    )?;
    let ready_pool_canisters = checked_root_count_sum(
        preflight,
        |root| root.remaining_pool_canisters,
        "remaining ready pool",
    )?;
    let role_canisters = 1_u32
        .checked_add(root_canisters)
        .and_then(|value| value.checked_add(root_canisters))
        .and_then(|value| value.checked_add(component_canisters))
        .ok_or(FreshFleetDeploymentPlanError::CountOverflow {
            subject: "role Canister",
        })?;
    let total_canisters = role_canisters.checked_add(ready_pool_canisters).ok_or(
        FreshFleetDeploymentPlanError::CountOverflow {
            subject: "total Canister",
        },
    )?;

    Ok(FreshFleetCanisterCountsV1 {
        coordinator_canisters: 1,
        root_canisters,
        wasm_store_canisters: root_canisters,
        component_canisters,
        ready_pool_canisters,
        role_canisters,
        total_canisters,
    })
}

fn checked_root_count_sum(
    preflight: &FreshFleetPreflightV1,
    count: impl Fn(&FreshFleetSubnetRootPlanV1) -> u32,
    subject: &'static str,
) -> Result<u32, FreshFleetDeploymentPlanError> {
    preflight
        .fleet_subnet_roots
        .iter()
        .try_fold(0_u32, |total, root| {
            total
                .checked_add(count(root))
                .ok_or(FreshFleetDeploymentPlanError::CountOverflow { subject })
        })
}

fn compile_funding_requirements(
    preflight: &FreshFleetPreflightV1,
    counts: FreshFleetCanisterCountsV1,
) -> Result<Vec<FreshFleetFundingRequirementV1>, FreshFleetDeploymentPlanError> {
    let mut requirements = vec![funding_requirement(
        "coordinator_creation",
        "Fleet Coordinator".to_string(),
        FreshFleetFundingPayerV1::Operator,
        1,
        &preflight.coordinator.creation_funding,
    )?];

    for root in &preflight.fleet_subnet_roots {
        requirements.push(funding_requirement(
            "root_creation",
            format!("Fleet Subnet Root {}", root.placement_subnet),
            FreshFleetFundingPayerV1::Operator,
            1,
            &root.root_creation_funding,
        )?);
        requirements.push(funding_requirement(
            "wasm_store_creation",
            format!("Wasm Store for Fleet Subnet Root {}", root.placement_subnet),
            FreshFleetFundingPayerV1::Operator,
            1,
            &root.wasm_store_creation_funding,
        )?);
        if root.pool_canister_creations > 0 {
            let pool_funding = PlannedCanisterCreationFunding::Cycles {
                cycles: root.limits.canister_pool.canister_cycles.to_u128(),
            };
            requirements.push(funding_requirement(
                "pool_creation",
                format!(
                    "Canister pool for Fleet Subnet Root {}",
                    root.placement_subnet
                ),
                FreshFleetFundingPayerV1::FleetSubnetRoot,
                root.pool_canister_creations,
                &pool_funding,
            )?);
        }
    }
    let operator_creation_count = counts
        .coordinator_canisters
        .checked_add(counts.root_canisters)
        .and_then(|count| count.checked_add(counts.wasm_store_canisters))
        .ok_or(FreshFleetDeploymentPlanError::CountOverflow {
            subject: "operator-created infrastructure Canister",
        })?;
    requirements.push(funding_requirement(
        "cycles_ledger_creation_fee",
        "Operator-created infrastructure Canisters".to_string(),
        FreshFleetFundingPayerV1::Operator,
        operator_creation_count,
        &PlannedCanisterCreationFunding::Cycles {
            cycles: CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES,
        },
    )?);
    requirements.sort();
    Ok(requirements)
}

fn funding_requirement(
    category: &str,
    owner: String,
    payer: FreshFleetFundingPayerV1,
    canister_count: u32,
    per_canister: &PlannedCanisterCreationFunding,
) -> Result<FreshFleetFundingRequirementV1, FreshFleetDeploymentPlanError> {
    let maximum = multiply_funding(per_canister, canister_count, &owner)?;
    Ok(FreshFleetFundingRequirementV1 {
        category: category.to_string(),
        owner,
        payer,
        canister_count,
        per_canister: per_canister.clone(),
        maximum,
    })
}

fn multiply_funding(
    funding: &PlannedCanisterCreationFunding,
    count: u32,
    subject: &str,
) -> Result<PlannedCanisterCreationFunding, FreshFleetDeploymentPlanError> {
    match funding {
        PlannedCanisterCreationFunding::Cycles { cycles } => cycles
            .checked_mul(u128::from(count))
            .map(|cycles| PlannedCanisterCreationFunding::Cycles { cycles }),
        PlannedCanisterCreationFunding::Icp { e8s } => e8s
            .checked_mul(u64::from(count))
            .map(|e8s| PlannedCanisterCreationFunding::Icp { e8s }),
    }
    .ok_or_else(|| FreshFleetDeploymentPlanError::FundingOverflow {
        subject: subject.to_string(),
    })
}

fn maximum_operator_debit(
    requirements: &[FreshFleetFundingRequirementV1],
) -> Result<PlannedCanisterCreationFunding, FreshFleetDeploymentPlanError> {
    requirements
        .iter()
        .filter(|requirement| requirement.payer == FreshFleetFundingPayerV1::Operator)
        .try_fold(None, |total, requirement| {
            match (total, &requirement.maximum) {
                (None, value) => Ok(Some(value.clone())),
                (
                    Some(PlannedCanisterCreationFunding::Cycles { cycles: left }),
                    PlannedCanisterCreationFunding::Cycles { cycles: right },
                ) => left
                    .checked_add(*right)
                    .map(|cycles| Some(PlannedCanisterCreationFunding::Cycles { cycles }))
                    .ok_or_else(|| FreshFleetDeploymentPlanError::FundingOverflow {
                        subject: "maximum operator debit".to_string(),
                    }),
                (
                    Some(PlannedCanisterCreationFunding::Icp { e8s: left }),
                    PlannedCanisterCreationFunding::Icp { e8s: right },
                ) => left
                    .checked_add(*right)
                    .map(|e8s| Some(PlannedCanisterCreationFunding::Icp { e8s }))
                    .ok_or_else(|| FreshFleetDeploymentPlanError::FundingOverflow {
                        subject: "maximum operator debit".to_string(),
                    }),
                (Some(_), _) => Err(FreshFleetDeploymentPlanError::MixedOperatorFunding),
            }
        })?
        .ok_or(FreshFleetDeploymentPlanError::MixedOperatorFunding)
}

fn validate_operator_balance(
    balance: &PlannedCanisterCreationFunding,
    debit: &PlannedCanisterCreationFunding,
) -> Result<(), FreshFleetDeploymentPlanError> {
    let sufficient = match (balance, debit) {
        (
            PlannedCanisterCreationFunding::Cycles { cycles: balance },
            PlannedCanisterCreationFunding::Cycles { cycles: debit },
        ) => balance >= debit,
        (
            PlannedCanisterCreationFunding::Icp { e8s: balance },
            PlannedCanisterCreationFunding::Icp { e8s: debit },
        ) => balance >= debit,
        _ => return Err(FreshFleetDeploymentPlanError::OperatorBalanceUnitMismatch),
    };
    if sufficient {
        Ok(())
    } else {
        Err(FreshFleetDeploymentPlanError::InsufficientOperatorBalance)
    }
}

fn plan_digest(
    preflight: &FreshFleetPreflightV1,
    authority: &FreshFleetDecisionAuthorityV1,
    counts: FreshFleetCanisterCountsV1,
    funding_requirements: &[FreshFleetFundingRequirementV1],
    maximum_operator_debit: &PlannedCanisterCreationFunding,
) -> Result<String, FreshFleetDeploymentPlanError> {
    let input = FreshFleetDeploymentPlanDigestInputV1 {
        schema_version: FRESH_FLEET_DEPLOYMENT_PLAN_SCHEMA_VERSION,
        preflight,
        authority: FreshFleetDecisionAuthorityDigestV1 {
            app_config_sha256: &authority.app_config_sha256,
            requested_environment: &authority.requested_environment,
            canonical_network_id: &authority.canonical_network_id,
            fleet_input_schema_version: authority.fleet_input_schema_version,
            fleet_input_sha256: &authority.fleet_input_sha256,
            release_source: &authority.release_source,
            catalog: &authority.catalog,
            operator: FreshFleetOperatorAuthorityDigestV1 {
                principal: &authority.operator.principal,
                funding_account: &authority.operator.funding_account,
            },
        },
        counts,
        funding_requirements,
        maximum_operator_debit,
        operator_balance_sufficient: true,
    };
    let bytes =
        serde_json::to_vec(&input).map_err(FreshFleetDeploymentPlanError::PlanSerialization)?;
    let mut hasher = Sha256::new();
    hasher.update(PLAN_DIGEST_DOMAIN);
    hasher.update(bytes);
    Ok(hex_bytes(hasher.finalize()))
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FreshFleetDeploymentPlanDigestInputV1<'a> {
    schema_version: u16,
    preflight: &'a FreshFleetPreflightV1,
    authority: FreshFleetDecisionAuthorityDigestV1<'a>,
    counts: FreshFleetCanisterCountsV1,
    funding_requirements: &'a [FreshFleetFundingRequirementV1],
    maximum_operator_debit: &'a PlannedCanisterCreationFunding,
    operator_balance_sufficient: bool,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FreshFleetDecisionAuthorityDigestV1<'a> {
    app_config_sha256: &'a str,
    requested_environment: &'a str,
    canonical_network_id: &'a canic_core::ids::CanonicalNetworkId,
    fleet_input_schema_version: u32,
    fleet_input_sha256: &'a str,
    release_source: &'a FreshFleetReleaseSourceV1,
    catalog: &'a FreshFleetCatalogEvidenceV1,
    operator: FreshFleetOperatorAuthorityDigestV1<'a>,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct FreshFleetOperatorAuthorityDigestV1<'a> {
    principal: &'a str,
    funding_account: &'a str,
}
