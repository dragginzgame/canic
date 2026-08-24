//! Module: model::fleet_admission_policy
//!
//! Responsibility: own canonical Fleet admission policy invariants and structural bounds.
//! Does not own: transport decoding, hashing, storage, orchestration, or endpoint decisions.
//! Boundary: ops supplies exact layer-neutral policy shapes plus independently computed digests.

use crate::ids::{
    FLEET_ADMISSION_INITIAL_GENERATION, FLEET_ADMISSION_SCHEMA_VERSION, FleetAdmissionRule,
    FleetAdmissionSelector, MAX_FLEET_ADMISSION_PRINCIPALS,
    MAX_FLEET_ADMISSION_RULE_PRINCIPAL_REFERENCES, MAX_FLEET_ADMISSION_RULES,
};
use candid::Principal;
use thiserror::Error as ThisError;

/// Exact invariant rejected while admitting a template or installed policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ThisError)]
pub enum FleetAdmissionPolicyValidationError {
    #[error("Fleet admission schema version is unsupported")]
    UnsupportedSchema,
    #[error("Fleet admission generation must be positive")]
    GenerationZero,
    #[error("generation-one admission policy must use the initial generation")]
    InitialGenerationMismatch,
    #[error("Fleet admission policy must contain between one and 256 Principals")]
    FleetPrincipalCountInvalid,
    #[error("Fleet admission Principals are not strictly canonical")]
    FleetPrincipalsNonCanonical,
    #[error("Fleet admission must not contain the anonymous Principal")]
    AnonymousPrincipal,
    #[error("Fleet admission narrower-rule count exceeds 32")]
    RuleCountExceeded,
    #[error("Fleet admission rules are not strictly canonical")]
    RulesNonCanonical,
    #[error("the Fleet-wide selector must not appear as a narrower rule")]
    FleetSelectorInNarrowerRule,
    #[error("Fleet admission narrower-rule Principal references exceed 128")]
    RulePrincipalReferenceCountExceeded,
    #[error("Fleet admission rule Principals are not strictly canonical")]
    RulePrincipalsNonCanonical,
    #[error("Fleet admission narrower rule attempts to widen the Fleet set")]
    RuleWidensFleet,
    #[error("Fleet admission digest is invalid")]
    DigestMismatch,
}

/// DTO-free policy facts supplied to the model-owned validator.
pub struct FleetAdmissionPolicyValidationInput<'a> {
    pub schema_version: u16,
    pub generation: Option<u64>,
    pub fleet_principals: &'a [Principal],
    pub rules: &'a [FleetAdmissionRule],
    pub digest_matches: bool,
}

/// Validate one complete template or installed policy without reading ambient state.
pub fn validate_fleet_admission_policy(
    input: &FleetAdmissionPolicyValidationInput<'_>,
) -> Result<(), FleetAdmissionPolicyValidationError> {
    if input.schema_version != FLEET_ADMISSION_SCHEMA_VERSION {
        return Err(FleetAdmissionPolicyValidationError::UnsupportedSchema);
    }
    if input.generation == Some(0) {
        return Err(FleetAdmissionPolicyValidationError::GenerationZero);
    }
    if input.fleet_principals.is_empty()
        || input.fleet_principals.len() > MAX_FLEET_ADMISSION_PRINCIPALS
    {
        return Err(FleetAdmissionPolicyValidationError::FleetPrincipalCountInvalid);
    }
    validate_principals(
        input.fleet_principals,
        FleetAdmissionPolicyValidationError::FleetPrincipalsNonCanonical,
    )?;
    if input.rules.len() > MAX_FLEET_ADMISSION_RULES {
        return Err(FleetAdmissionPolicyValidationError::RuleCountExceeded);
    }
    if input
        .rules
        .windows(2)
        .any(|rules| rules[0].selector >= rules[1].selector)
    {
        return Err(FleetAdmissionPolicyValidationError::RulesNonCanonical);
    }
    let reference_count = input.rules.iter().try_fold(0_usize, |count, rule| {
        if rule.selector == FleetAdmissionSelector::Fleet {
            return Err(FleetAdmissionPolicyValidationError::FleetSelectorInNarrowerRule);
        }
        validate_principals(
            &rule.principals,
            FleetAdmissionPolicyValidationError::RulePrincipalsNonCanonical,
        )?;
        if rule
            .principals
            .iter()
            .any(|principal| input.fleet_principals.binary_search(principal).is_err())
        {
            return Err(FleetAdmissionPolicyValidationError::RuleWidensFleet);
        }
        count
            .checked_add(rule.principals.len())
            .ok_or(FleetAdmissionPolicyValidationError::RulePrincipalReferenceCountExceeded)
    })?;
    if reference_count > MAX_FLEET_ADMISSION_RULE_PRINCIPAL_REFERENCES {
        return Err(FleetAdmissionPolicyValidationError::RulePrincipalReferenceCountExceeded);
    }
    if !input.digest_matches {
        return Err(FleetAdmissionPolicyValidationError::DigestMismatch);
    }
    Ok(())
}

/// Require the exact generation used for a fresh-Fleet policy binding.
pub const fn validate_initial_fleet_admission_generation(
    generation: u64,
) -> Result<(), FleetAdmissionPolicyValidationError> {
    if generation == FLEET_ADMISSION_INITIAL_GENERATION {
        Ok(())
    } else {
        Err(FleetAdmissionPolicyValidationError::InitialGenerationMismatch)
    }
}

fn validate_principals(
    principals: &[Principal],
    noncanonical: FleetAdmissionPolicyValidationError,
) -> Result<(), FleetAdmissionPolicyValidationError> {
    if principals
        .iter()
        .any(|principal| principal == &Principal::anonymous())
    {
        return Err(FleetAdmissionPolicyValidationError::AnonymousPrincipal);
    }
    if principals.windows(2).any(|items| items[0] >= items[1]) {
        return Err(noncanonical);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
