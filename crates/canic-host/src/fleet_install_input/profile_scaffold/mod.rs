//! Module: fleet_install_input::profile_scaffold
//!
//! Responsibility: materialize exact node-scaled funding-profile values for authoring.
//! Does not own: Fleet topology, operator identity, live balances, or install admission.
//! Boundary: output is deterministic funding authority derived only from explicit node counts.

#[cfg(test)]
use super::TRILLION_CYCLES;
use super::{
    PROFILE_ROUNDING_CYCLES, STANDARD_SUBNET_NODE_COUNT, funding_profile_baselines,
    scale_profile_cycles,
};
use crate::fleet_install_plan::CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES;

use canic_core::ids::{FleetFundingProfile, MAX_FLEET_ROOT_FUNDING_SLOTS};
use thiserror::Error as ThisError;

const THIRTY_DAYS_SECS: u64 = 30 * 24 * 60 * 60;
const NINETY_DAYS_SECS: u64 = 90 * 24 * 60 * 60;

///
/// FleetFundingProfileScaffold
///
/// Exact funding-only profile materialization used by the offline CLI scaffold.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetFundingProfileScaffold {
    pub profile: FleetFundingProfile,
    pub coordinator_node_count: u64,
    pub coordinator: FleetFundingProfileCoordinatorScaffold,
    pub roots: Vec<FleetFundingProfileRootScaffold>,
    pub operator_creation_amount_cycles: u128,
    pub operator_creation_count: u32,
    pub operator_creation_fee_cycles: u128,
    pub maximum_operator_debit_cycles: u128,
    pub formulas: Vec<FleetFundingProfileFormula>,
}

///
/// FleetFundingProfileFormula
///
/// One exact arithmetic derivation rendered by the authoring scaffold.
///
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetFundingProfileFormula {
    pub field: String,
    pub expression: String,
    pub result: u128,
}

///
/// FleetFundingProfileCoordinatorScaffold
///
/// Exact Coordinator funding values selected by one profile materialization.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetFundingProfileCoordinatorScaffold {
    pub creation_cycles: u128,
    pub minimum_reserve_cycles: u128,
    pub window_secs: u64,
    pub maximum_cycles: u128,
    pub maximum_automatic_grants: u32,
    pub maximum_automatic_cycles: u128,
}

///
/// FleetFundingProfileRootScaffold
///
/// Exact per-Root funding values selected from one Registry-observed node count.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FleetFundingProfileRootScaffold {
    pub node_count: u64,
    pub root_creation_cycles: u128,
    pub wasm_store_creation_cycles: u128,
    pub request_threshold_cycles: u128,
    pub target_balance_cycles: u128,
    pub cooldown_secs: u64,
    pub window_secs: u64,
    pub maximum_cycles: u128,
    pub maximum_automatic_grants: u32,
    pub maximum_automatic_cycles: u128,
}

///
/// FleetFundingProfileScaffoldError
///
/// Typed rejection while materializing one funding-only scaffold.
///

#[derive(Debug, ThisError)]
pub enum FleetFundingProfileScaffoldError {
    #[error("funding-profile scaffold requires at least one Fleet Subnet Root")]
    MissingRoots,

    #[error("funding-profile scaffold root count {actual} exceeds the supported maximum {maximum}")]
    RootCountOverflow { actual: usize, maximum: usize },

    #[error(
        "single_subnet funding-profile scaffold requires exactly one Root node count equal to the Coordinator node count"
    )]
    SingleSubnetTopology,

    #[error("funding-profile arithmetic overflowed while resolving {field}")]
    Overflow { field: &'static str },

    #[error("{owner} node count must be positive")]
    ZeroNodeCount { owner: String },
}

struct OperatorFundingScaffold {
    creation_amount_cycles: u128,
    creation_count: u32,
    creation_fee_cycles: u128,
    maximum_debit_cycles: u128,
}

/// Materialize one exact funding-only profile without reading identity or balance state.
pub fn scaffold_fleet_funding_profile(
    profile: FleetFundingProfile,
    coordinator_node_count: u64,
    root_node_counts: &[u64],
) -> Result<FleetFundingProfileScaffold, FleetFundingProfileScaffoldError> {
    validate_scaffold_topology(profile, coordinator_node_count, root_node_counts)?;
    let roots = scaffold_roots(profile, root_node_counts)?;
    let coordinator = scaffold_coordinator(profile, coordinator_node_count, &roots)?;
    let operator = scaffold_operator_funding(&coordinator, &roots)?;
    let mut scaffold = FleetFundingProfileScaffold {
        profile,
        coordinator_node_count,
        coordinator,
        roots,
        operator_creation_amount_cycles: operator.creation_amount_cycles,
        operator_creation_count: operator.creation_count,
        operator_creation_fee_cycles: operator.creation_fee_cycles,
        maximum_operator_debit_cycles: operator.maximum_debit_cycles,
        formulas: Vec::new(),
    };
    scaffold.formulas = scaffold_formulas(&scaffold);
    Ok(scaffold)
}

fn scaffold_coordinator(
    profile: FleetFundingProfile,
    coordinator_node_count: u64,
    roots: &[FleetFundingProfileRootScaffold],
) -> Result<FleetFundingProfileCoordinatorScaffold, FleetFundingProfileScaffoldError> {
    let root_targets = roots.iter().map(|root| root.target_balance_cycles);
    let baselines = funding_profile_baselines(profile);
    let coordinator_reserve = scaled(
        baselines.coordinator_reserve_cycles,
        coordinator_node_count,
        "Coordinator minimum reserve",
    )?;
    let coordinator_window_maximum = match profile {
        FleetFundingProfile::SingleSubnet => roots[0].target_balance_cycles,
        FleetFundingProfile::PreviewMultiSubnet | FleetFundingProfile::MultiSubnet => {
            checked_sum(root_targets.clone(), "Fleet window maximum")?
        }
    };
    let (maximum_automatic_grants, maximum_automatic_cycles) = match profile {
        FleetFundingProfile::SingleSubnet => (
            roots[0].maximum_automatic_grants,
            roots[0].maximum_automatic_cycles,
        ),
        FleetFundingProfile::PreviewMultiSubnet => {
            let largest_target = root_targets
                .clone()
                .max()
                .ok_or(FleetFundingProfileScaffoldError::MissingRoots)?;
            (
                2,
                checked_mul(largest_target, 2, "Fleet automatic cycle cap")?,
            )
        }
        FleetFundingProfile::MultiSubnet => (
            checked_root_grants(roots)?,
            checked_sum(
                roots.iter().map(|root| root.maximum_automatic_cycles),
                "Fleet automatic cycle cap",
            )?,
        ),
    };
    let creation_cycles = match profile {
        FleetFundingProfile::SingleSubnet => scaled(
            baselines.required_coordinator_creation_cycles(),
            coordinator_node_count,
            "Coordinator creation funding",
        )?,
        FleetFundingProfile::PreviewMultiSubnet => coordinator_reserve
            .checked_add(maximum_automatic_cycles)
            .ok_or(FleetFundingProfileScaffoldError::Overflow {
                field: "Coordinator creation funding",
            })?,
        FleetFundingProfile::MultiSubnet => coordinator_reserve
            .checked_add(checked_mul(
                checked_sum(root_targets, "Coordinator creation funding")?,
                2,
                "Coordinator creation funding",
            )?)
            .ok_or(FleetFundingProfileScaffoldError::Overflow {
                field: "Coordinator creation funding",
            })?,
    };
    Ok(FleetFundingProfileCoordinatorScaffold {
        creation_cycles,
        minimum_reserve_cycles: coordinator_reserve,
        window_secs: NINETY_DAYS_SECS,
        maximum_cycles: coordinator_window_maximum,
        maximum_automatic_grants,
        maximum_automatic_cycles,
    })
}

fn scaffold_operator_funding(
    coordinator: &FleetFundingProfileCoordinatorScaffold,
    roots: &[FleetFundingProfileRootScaffold],
) -> Result<OperatorFundingScaffold, FleetFundingProfileScaffoldError> {
    let creation_amount_cycles = checked_sum(
        operator_creation_amounts(coordinator, roots),
        "operator creation amount",
    )?;
    let root_count = u32::try_from(roots.len()).map_err(|_| {
        FleetFundingProfileScaffoldError::RootCountOverflow {
            actual: roots.len(),
            maximum: MAX_FLEET_ROOT_FUNDING_SLOTS,
        }
    })?;
    let creation_count = root_count
        .checked_mul(2)
        .and_then(|count| count.checked_add(1))
        .ok_or(FleetFundingProfileScaffoldError::Overflow {
            field: "operator creation count",
        })?;
    let creation_fee_cycles = CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES
        .checked_mul(u128::from(creation_count))
        .ok_or(FleetFundingProfileScaffoldError::Overflow {
            field: "operator creation fees",
        })?;
    let maximum_debit_cycles = creation_amount_cycles
        .checked_add(creation_fee_cycles)
        .ok_or(FleetFundingProfileScaffoldError::Overflow {
            field: "maximum operator debit",
        })?;
    Ok(OperatorFundingScaffold {
        creation_amount_cycles,
        creation_count,
        creation_fee_cycles,
        maximum_debit_cycles,
    })
}

fn scaffold_root(
    profile: FleetFundingProfile,
    node_count: u64,
) -> Result<FleetFundingProfileRootScaffold, FleetFundingProfileScaffoldError> {
    let baselines = funding_profile_baselines(profile);
    let target_balance_cycles = scaled(
        baselines.root.target_balance_cycles,
        node_count,
        "Root target balance",
    )?;
    Ok(FleetFundingProfileRootScaffold {
        node_count,
        root_creation_cycles: scaled(
            baselines.root.root_creation_cycles,
            node_count,
            "Root creation funding",
        )?,
        wasm_store_creation_cycles: scaled(
            baselines.root.store_creation_cycles,
            node_count,
            "Wasm Store creation funding",
        )?,
        request_threshold_cycles: scaled(
            baselines.root.request_threshold_cycles,
            node_count,
            "Root request threshold",
        )?,
        target_balance_cycles,
        cooldown_secs: THIRTY_DAYS_SECS,
        window_secs: NINETY_DAYS_SECS,
        maximum_cycles: target_balance_cycles,
        maximum_automatic_grants: baselines.root_maximum_automatic_grants,
        maximum_automatic_cycles: checked_mul(
            target_balance_cycles,
            baselines.root_maximum_automatic_grants,
            "Root automatic cycle cap",
        )?,
    })
}

fn scaffold_roots(
    profile: FleetFundingProfile,
    node_counts: &[u64],
) -> Result<Vec<FleetFundingProfileRootScaffold>, FleetFundingProfileScaffoldError> {
    node_counts
        .iter()
        .enumerate()
        .map(|(index, node_count)| {
            validate_node_count(&format!("Root {}", index + 1), *node_count)?;
            scaffold_root(profile, *node_count)
        })
        .collect()
}

fn validate_scaffold_topology(
    profile: FleetFundingProfile,
    coordinator_node_count: u64,
    root_node_counts: &[u64],
) -> Result<(), FleetFundingProfileScaffoldError> {
    validate_node_count("Coordinator", coordinator_node_count)?;
    if root_node_counts.is_empty() {
        return Err(FleetFundingProfileScaffoldError::MissingRoots);
    }
    if root_node_counts.len() > MAX_FLEET_ROOT_FUNDING_SLOTS {
        return Err(FleetFundingProfileScaffoldError::RootCountOverflow {
            actual: root_node_counts.len(),
            maximum: MAX_FLEET_ROOT_FUNDING_SLOTS,
        });
    }
    if profile == FleetFundingProfile::SingleSubnet
        && (root_node_counts.len() != 1 || root_node_counts[0] != coordinator_node_count)
    {
        return Err(FleetFundingProfileScaffoldError::SingleSubnetTopology);
    }
    Ok(())
}

fn operator_creation_amounts<'a>(
    coordinator: &'a FleetFundingProfileCoordinatorScaffold,
    roots: &'a [FleetFundingProfileRootScaffold],
) -> impl Iterator<Item = u128> + 'a {
    std::iter::once(coordinator.creation_cycles).chain(
        roots
            .iter()
            .flat_map(|root| [root.root_creation_cycles, root.wasm_store_creation_cycles]),
    )
}

fn scaled(
    standard_cycles: u128,
    node_count: u64,
    field: &'static str,
) -> Result<u128, FleetFundingProfileScaffoldError> {
    scale_profile_cycles(standard_cycles, node_count, field)
        .map_err(|_| FleetFundingProfileScaffoldError::Overflow { field })
}

fn checked_sum(
    values: impl IntoIterator<Item = u128>,
    field: &'static str,
) -> Result<u128, FleetFundingProfileScaffoldError> {
    values.into_iter().try_fold(0_u128, |total, value| {
        total
            .checked_add(value)
            .ok_or(FleetFundingProfileScaffoldError::Overflow { field })
    })
}

fn checked_mul(
    value: u128,
    count: u32,
    field: &'static str,
) -> Result<u128, FleetFundingProfileScaffoldError> {
    value
        .checked_mul(u128::from(count))
        .ok_or(FleetFundingProfileScaffoldError::Overflow { field })
}

fn checked_root_grants(
    roots: &[FleetFundingProfileRootScaffold],
) -> Result<u32, FleetFundingProfileScaffoldError> {
    roots.iter().try_fold(0_u32, |total, root| {
        total.checked_add(root.maximum_automatic_grants).ok_or(
            FleetFundingProfileScaffoldError::Overflow {
                field: "Fleet automatic grant cap",
            },
        )
    })
}

fn validate_node_count(
    owner: &str,
    node_count: u64,
) -> Result<(), FleetFundingProfileScaffoldError> {
    if node_count == 0 {
        Err(FleetFundingProfileScaffoldError::ZeroNodeCount {
            owner: owner.to_string(),
        })
    } else {
        Ok(())
    }
}

fn scaffold_formulas(scaffold: &FleetFundingProfileScaffold) -> Vec<FleetFundingProfileFormula> {
    let mut formulas = vec![scaled_formula(
        "coordinator.minimum_reserve_cycles",
        funding_profile_baselines(scaffold.profile).coordinator_reserve_cycles,
        scaffold.coordinator_node_count,
        scaffold.coordinator.minimum_reserve_cycles,
    )];
    for (index, root) in scaffold.roots.iter().enumerate() {
        formulas.extend(root_formulas(scaffold.profile, index, root));
    }
    formulas.extend(coordinator_formulas(scaffold));
    formulas.extend(operator_formulas(scaffold));
    formulas
}

fn root_formulas(
    profile: FleetFundingProfile,
    index: usize,
    root: &FleetFundingProfileRootScaffold,
) -> [FleetFundingProfileFormula; 6] {
    let prefix = format!("fleet_subnet_roots[{}]", index + 1);
    let baselines = funding_profile_baselines(profile);
    [
        scaled_formula(
            &format!("{prefix}.root_creation_funding.cycles"),
            baselines.root.root_creation_cycles,
            root.node_count,
            root.root_creation_cycles,
        ),
        scaled_formula(
            &format!("{prefix}.wasm_store_creation_funding.cycles"),
            baselines.root.store_creation_cycles,
            root.node_count,
            root.wasm_store_creation_cycles,
        ),
        scaled_formula(
            &format!("{prefix}.root_funding.request_threshold"),
            baselines.root.request_threshold_cycles,
            root.node_count,
            root.request_threshold_cycles,
        ),
        scaled_formula(
            &format!("{prefix}.root_funding.target_balance"),
            baselines.root.target_balance_cycles,
            root.node_count,
            root.target_balance_cycles,
        ),
        FleetFundingProfileFormula {
            field: format!("{prefix}.root_funding.maximum_cycles"),
            expression: root.target_balance_cycles.to_string(),
            result: root.maximum_cycles,
        },
        FleetFundingProfileFormula {
            field: format!("{prefix}.root_funding.maximum_automatic_cycles"),
            expression: format!(
                "{} * {}",
                root.target_balance_cycles, root.maximum_automatic_grants
            ),
            result: root.maximum_automatic_cycles,
        },
    ]
}

fn coordinator_formulas(scaffold: &FleetFundingProfileScaffold) -> [FleetFundingProfileFormula; 3] {
    let coordinator = &scaffold.coordinator;
    let roots = &scaffold.roots;
    let root_targets = roots
        .iter()
        .map(|root| root.target_balance_cycles.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    let maximum_cycles = FleetFundingProfileFormula {
        field: "coordinator.root_funding.maximum_cycles".to_string(),
        expression: if scaffold.profile == FleetFundingProfile::SingleSubnet {
            roots[0].target_balance_cycles.to_string()
        } else {
            root_targets.clone()
        },
        result: coordinator.maximum_cycles,
    };
    let maximum_automatic_cycles = FleetFundingProfileFormula {
        field: "coordinator.root_funding.maximum_automatic_cycles".to_string(),
        expression: match scaffold.profile {
            FleetFundingProfile::SingleSubnet => roots[0].maximum_automatic_cycles.to_string(),
            FleetFundingProfile::PreviewMultiSubnet => format!(
                "2 * max({})",
                roots
                    .iter()
                    .map(|root| root.target_balance_cycles.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FleetFundingProfile::MultiSubnet => roots
                .iter()
                .map(|root| root.maximum_automatic_cycles.to_string())
                .collect::<Vec<_>>()
                .join(" + "),
        },
        result: coordinator.maximum_automatic_cycles,
    };
    let creation_cycles = match scaffold.profile {
        FleetFundingProfile::SingleSubnet => scaled_formula(
            "coordinator.creation_funding.cycles",
            funding_profile_baselines(scaffold.profile).required_coordinator_creation_cycles(),
            scaffold.coordinator_node_count,
            coordinator.creation_cycles,
        ),
        FleetFundingProfile::PreviewMultiSubnet => FleetFundingProfileFormula {
            field: "coordinator.creation_funding.cycles".to_string(),
            expression: format!(
                "{} + {}",
                coordinator.minimum_reserve_cycles, coordinator.maximum_automatic_cycles
            ),
            result: coordinator.creation_cycles,
        },
        FleetFundingProfile::MultiSubnet => FleetFundingProfileFormula {
            field: "coordinator.creation_funding.cycles".to_string(),
            expression: format!(
                "{} + 2 * ({root_targets})",
                coordinator.minimum_reserve_cycles
            ),
            result: coordinator.creation_cycles,
        },
    };
    [maximum_cycles, maximum_automatic_cycles, creation_cycles]
}

fn operator_formulas(scaffold: &FleetFundingProfileScaffold) -> [FleetFundingProfileFormula; 3] {
    let creation_amounts = operator_creation_amounts(&scaffold.coordinator, &scaffold.roots)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(" + ");
    [
        FleetFundingProfileFormula {
            field: "operator.creation_amount_cycles".to_string(),
            expression: creation_amounts,
            result: scaffold.operator_creation_amount_cycles,
        },
        FleetFundingProfileFormula {
            field: "operator.creation_fee_cycles".to_string(),
            expression: format!(
                "{} * {CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES}",
                scaffold.operator_creation_count
            ),
            result: scaffold.operator_creation_fee_cycles,
        },
        FleetFundingProfileFormula {
            field: "operator.maximum_debit_cycles".to_string(),
            expression: format!(
                "{} + {}",
                scaffold.operator_creation_amount_cycles, scaffold.operator_creation_fee_cycles
            ),
            result: scaffold.maximum_operator_debit_cycles,
        },
    ]
}

fn scaled_formula(
    field: &str,
    standard_cycles: u128,
    node_count: u64,
    result: u128,
) -> FleetFundingProfileFormula {
    FleetFundingProfileFormula {
        field: field.to_string(),
        expression: format!(
            "ceil_to_{PROFILE_ROUNDING_CYCLES}({standard_cycles} * max({node_count}, {STANDARD_SUBNET_NODE_COUNT}) / {STANDARD_SUBNET_NODE_COUNT})"
        ),
        result,
    }
}

// Keep these authority constants visible in formula renderers without duplicating values.
pub const STANDARD_PROFILE_NODE_COUNT: u64 = STANDARD_SUBNET_NODE_COUNT;
pub const PROFILE_CYCLE_ROUNDING_QUANTUM: u128 = PROFILE_ROUNDING_CYCLES;

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_all_standard_thirteen_node_profiles() {
        let single = scaffold_fleet_funding_profile(FleetFundingProfile::SingleSubnet, 13, &[13])
            .expect("single-Subnet profile");
        assert_eq!(single.coordinator.creation_cycles, 100 * TRILLION_CYCLES);
        assert_eq!(
            single.coordinator.minimum_reserve_cycles,
            30 * TRILLION_CYCLES
        );
        assert_eq!(single.roots[0].root_creation_cycles, 30 * TRILLION_CYCLES);
        assert_eq!(
            single.roots[0].wasm_store_creation_cycles,
            10 * TRILLION_CYCLES
        );

        let preview =
            scaffold_fleet_funding_profile(FleetFundingProfile::PreviewMultiSubnet, 13, &[13])
                .expect("preview multi-Subnet profile");
        assert_eq!(preview.coordinator.creation_cycles, 140 * TRILLION_CYCLES);
        assert_eq!(
            preview.coordinator.minimum_reserve_cycles,
            80 * TRILLION_CYCLES
        );
        assert_eq!(
            preview.coordinator.maximum_automatic_cycles,
            60 * TRILLION_CYCLES
        );

        let professional =
            scaffold_fleet_funding_profile(FleetFundingProfile::MultiSubnet, 13, &[13])
                .expect("professional multi-Subnet profile");
        assert_eq!(
            professional.coordinator.creation_cycles,
            4_000 * TRILLION_CYCLES
        );
        assert_eq!(
            professional.roots[0].root_creation_cycles,
            1_000 * TRILLION_CYCLES
        );
        assert_eq!(
            professional.roots[0].wasm_store_creation_cycles,
            200 * TRILLION_CYCLES
        );
    }

    #[test]
    fn materializes_toko_node_scaling_and_fee_complete_debit() {
        let scaffold =
            scaffold_fleet_funding_profile(FleetFundingProfile::PreviewMultiSubnet, 34, &[13])
                .expect("Toko preview profile");

        assert_eq!(
            scaffold.coordinator.minimum_reserve_cycles,
            210 * TRILLION_CYCLES
        );
        assert_eq!(scaffold.coordinator.creation_cycles, 270 * TRILLION_CYCLES);
        assert_eq!(
            scaffold.operator_creation_amount_cycles,
            310 * TRILLION_CYCLES
        );
        assert_eq!(scaffold.operator_creation_count, 3);
        assert_eq!(
            scaffold.operator_creation_fee_cycles,
            3 * CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES
        );
        assert_eq!(
            scaffold.maximum_operator_debit_cycles,
            310 * TRILLION_CYCLES + 3 * CYCLES_LEDGER_CREATE_CANISTER_FEE_CYCLES
        );
    }

    #[test]
    fn single_subnet_scaffold_rejects_an_impossible_node_count_split() {
        assert!(matches!(
            scaffold_fleet_funding_profile(FleetFundingProfile::SingleSubnet, 34, &[13]),
            Err(FleetFundingProfileScaffoldError::SingleSubnetTopology)
        ));
    }
}
