//! Module: model::fleet_admission_policy::tests
//!
//! Responsibility: qualify canonical Fleet admission bounds and narrowing invariants.
//! Does not own: hashing, transport parsing, storage, or endpoint integration.

use super::*;
use crate::ids::{FleetAdmissionRule, FleetAdmissionSelector};

fn principal(index: usize) -> Principal {
    Principal::from_slice(
        &u16::try_from(index)
            .expect("test Principal index fits u16")
            .to_be_bytes(),
    )
}

fn input<'a>(
    principals: &'a [Principal],
    rules: &'a [FleetAdmissionRule],
) -> FleetAdmissionPolicyValidationInput<'a> {
    FleetAdmissionPolicyValidationInput {
        schema_version: FLEET_ADMISSION_SCHEMA_VERSION,
        generation: Some(FLEET_ADMISSION_INITIAL_GENERATION),
        fleet_principals: principals,
        rules,
        digest_matches: true,
    }
}

#[test]
fn bounded_canonical_policy_accepts_exact_limits() {
    let principals = (1..=MAX_FLEET_ADMISSION_PRINCIPALS)
        .map(principal)
        .collect::<Vec<_>>();
    let rules = (0..MAX_FLEET_ADMISSION_RULES)
        .map(|index| FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                format!("component_{index:02}")
                    .parse()
                    .expect("Component Spec ID"),
            ),
            principals: principals[index * 4..index * 4 + 4].to_vec(),
        })
        .collect::<Vec<_>>();

    validate_fleet_admission_policy(&input(&principals, &rules)).expect("exact bounds");
}

#[test]
fn first_excess_and_noncanonical_authority_fail_closed() {
    let mut principals = (1..=MAX_FLEET_ADMISSION_PRINCIPALS)
        .map(principal)
        .collect::<Vec<_>>();
    principals.push(principal(0));
    assert_eq!(
        validate_fleet_admission_policy(&input(&principals, &[])),
        Err(FleetAdmissionPolicyValidationError::FleetPrincipalCountInvalid)
    );

    let principals = [principal(1), principal(2)];
    let duplicate = [principal(1), principal(1)];
    assert_eq!(
        validate_fleet_admission_policy(&input(&duplicate, &[])),
        Err(FleetAdmissionPolicyValidationError::FleetPrincipalsNonCanonical)
    );

    let widening = [FleetAdmissionRule {
        selector: FleetAdmissionSelector::ComponentSpec("app".parse().expect("Component Spec ID")),
        principals: vec![principal(3)],
    }];
    assert_eq!(
        validate_fleet_admission_policy(&input(&principals, &widening)),
        Err(FleetAdmissionPolicyValidationError::RuleWidensFleet)
    );
}

#[test]
fn first_excess_rule_and_narrower_reference_bounds_fail_closed() {
    let principals = (1..=MAX_FLEET_ADMISSION_PRINCIPALS)
        .map(principal)
        .collect::<Vec<_>>();
    let excessive_rules = (0..=MAX_FLEET_ADMISSION_RULES)
        .map(|index| FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                format!("component_{index:02}")
                    .parse()
                    .expect("Component Spec ID"),
            ),
            principals: Vec::new(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_fleet_admission_policy(&input(&principals, &excessive_rules)),
        Err(FleetAdmissionPolicyValidationError::RuleCountExceeded)
    );

    let mut excessive_references = (0..MAX_FLEET_ADMISSION_RULES)
        .map(|index| FleetAdmissionRule {
            selector: FleetAdmissionSelector::ComponentSpec(
                format!("component_{index:02}")
                    .parse()
                    .expect("Component Spec ID"),
            ),
            principals: principals[index * 4..index * 4 + 4].to_vec(),
        })
        .collect::<Vec<_>>();
    excessive_references
        .last_mut()
        .expect("last rule")
        .principals
        .push(principals[200]);
    assert_eq!(
        validate_fleet_admission_policy(&input(&principals, &excessive_references)),
        Err(FleetAdmissionPolicyValidationError::RulePrincipalReferenceCountExceeded)
    );
}

#[test]
fn anonymous_fleet_selector_and_bad_digest_reject() {
    assert_eq!(
        validate_fleet_admission_policy(&input(&[Principal::anonymous()], &[])),
        Err(FleetAdmissionPolicyValidationError::AnonymousPrincipal)
    );

    let principals = [principal(1)];
    let fleet_rule = [FleetAdmissionRule {
        selector: FleetAdmissionSelector::Fleet,
        principals: principals.to_vec(),
    }];
    assert_eq!(
        validate_fleet_admission_policy(&input(&principals, &fleet_rule)),
        Err(FleetAdmissionPolicyValidationError::FleetSelectorInNarrowerRule)
    );

    let mut invalid_digest = input(&principals, &[]);
    invalid_digest.digest_matches = false;
    assert_eq!(
        validate_fleet_admission_policy(&invalid_digest),
        Err(FleetAdmissionPolicyValidationError::DigestMismatch)
    );
}
