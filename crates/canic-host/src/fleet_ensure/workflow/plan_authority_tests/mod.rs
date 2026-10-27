//! Focused proofs for moving funding estimates and exact executable authority.

use crate::fleet_ensure::{
    model::{CanisterDisposition, CanisterPlan, EnsureAction},
    ops::plan_authority::same_plan_authority,
    workflow::tests::estate_funding_plan,
};

fn funding_action() -> EnsureAction {
    EnsureAction::Fund {
        pool_funding: None,
        amount: 10,
        created_at_time: 1,
        expected_post_cycles: 30,
        funding_deficit_cycles: 5,
        funding_margin_cycles: 5,
        ledger: "ledger".into(),
        name: "root".into(),
        principal: "root-principal".into(),
    }
}

#[test]
fn funding_estimates_move_only_inside_canister_actions() {
    let mut retained = estate_funding_plan();
    retained.canisters.push(CanisterPlan {
        actions: vec![funding_action()],
        disposition: CanisterDisposition::Reuse,
        name: "root".into(),
        observed_cycles: 20,
        principal: Some("root-principal".into()),
    });
    retained.protocol_actions.push(funding_action());
    let mut current = retained.clone();
    current.plan_sha256 = "recomputed".into();
    current.canisters[0].observed_cycles -= 1;
    let EnsureAction::Fund {
        amount,
        funding_deficit_cycles,
        ..
    } = &mut current.canisters[0].actions[0]
    else {
        unreachable!()
    };
    *amount += 1;
    *funding_deficit_cycles += 1;
    assert!(same_plan_authority(&retained, &current));

    let mut changed_target = current.clone();
    let EnsureAction::Fund { principal, .. } = &mut changed_target.canisters[0].actions[0] else {
        unreachable!()
    };
    *principal = "another-principal".into();
    assert!(!same_plan_authority(&retained, &changed_target));

    let EnsureAction::Fund { amount, .. } = &mut current.protocol_actions[0] else {
        unreachable!()
    };
    *amount += 1;
    assert!(!same_plan_authority(&retained, &current));
}

#[test]
fn estate_estimates_move_without_replacing_creation_authority_or_membership() {
    let retained = estate_funding_plan();
    let mut current = retained.clone();
    current.conservation.observed_controlled_cycles -= 1;
    current.conservation.maximum_operator_debit_cycles += 1;
    let domain = &mut current.conservation.estate_funding_domains[0];
    domain.available_cycles = Some(39);
    domain.maximum_funding_cycles += 1;
    domain.root_principal = Some("reobserved-root".into());
    domain.shortfall_cycles += 1;
    assert!(same_plan_authority(&retained, &current));

    let mut changed_creation = current.clone();
    changed_creation.conservation.estate_funding_domains[0].creation_amount_cycles += 1;
    assert!(!same_plan_authority(&retained, &changed_creation));

    let mut changed_membership = current.clone();
    changed_membership.conservation.estate_funding_domains[0]
        .initial_pool_assets
        .push("unreviewed-asset".into());
    assert!(!same_plan_authority(&retained, &changed_membership));

    current.conservation.estate_funding_domains.clear();
    assert!(!same_plan_authority(&retained, &current));
}
