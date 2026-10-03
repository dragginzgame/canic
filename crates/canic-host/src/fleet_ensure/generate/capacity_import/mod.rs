//! Compile paired generator-input replacements for a reviewed capacity import.
//!
//! This boundary preserves existing estate identities and policy values. Publication,
//! authenticated observation and Root release belong to the import workflow.

#[cfg(test)]
pub(in crate::fleet_ensure) mod tests;

use crate::fleet_ensure::{
    generate::{
        EstateSeed, FleetGenerateError, FleetSource, MAX_GENERATOR_INPUT_BYTES,
        canonical_recovery_controllers, validate_identity_seed,
    },
    model::capacity_import::{CapacityImportPlanRecord, operation::CapacityImportPublicationKind},
    ops::capacity_import::{CapacityImportReviewError, verify_review},
    view::capacity_import::{CapacityImportDocumentView, CapacityImportInventoryView},
};
use candid::Principal;
use sha2_host::{Digest, Sha256};
use thiserror::Error;

/// Local projection failure leaves both original generator inputs untouched.
#[derive(Debug, Error)]
pub enum CapacityImportInventoryError {
    #[error("capacity import generator input exceeds its byte bound")]
    TooLarge,
    #[error("capacity import generator input is not UTF-8")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("capacity import generator input has an invalid current schema")]
    Decode(#[from] toml::de::Error),
    #[error("capacity import generator replacement cannot be encoded")]
    Encode(#[from] toml::ser::Error),
    #[error("capacity import generator inputs differ from the reviewed retained estate")]
    AuthorityMismatch,
    #[error("capacity import candidate already occurs in the destination seed")]
    AlreadyPresent,
    #[error(transparent)]
    Review(#[from] CapacityImportReviewError),
    #[error(transparent)]
    Generation(Box<FleetGenerateError>),
}

impl From<FleetGenerateError> for CapacityImportInventoryError {
    fn from(error: FleetGenerateError) -> Self {
        Self::Generation(Box::new(error))
    }
}

/// Add the exact reviewed sources to both maintained generator inputs without any writes.
///
/// Callers must retain and approve these byte bindings before the import's first effect.
/// An existing retained estate is required; bootstrap owns creation of its initial identity.
pub fn prepare_capacity_import_inventory(
    plan: &CapacityImportPlanRecord,
    policy_bytes: &[u8],
    seed_bytes: &[u8],
) -> Result<CapacityImportInventoryView, CapacityImportInventoryError> {
    prepare_inventory(
        plan,
        policy_bytes,
        seed_bytes,
        CapacityImportPublicationKind::ExtendEstate,
    )
}

/// Publish an initial held set already declared by the explicit infrastructure seed.
/// The bootstrap owner verifies its setup receipt before selecting this boundary.
pub fn prepare_initial_import_inventory(
    plan: &CapacityImportPlanRecord,
    policy_bytes: &[u8],
    seed_bytes: &[u8],
) -> Result<CapacityImportInventoryView, CapacityImportInventoryError> {
    prepare_inventory(
        plan,
        policy_bytes,
        seed_bytes,
        CapacityImportPublicationKind::InitializeEstate,
    )
}

fn prepare_inventory(
    plan: &CapacityImportPlanRecord,
    policy_bytes: &[u8],
    seed_bytes: &[u8],
    kind: CapacityImportPublicationKind,
) -> Result<CapacityImportInventoryView, CapacityImportInventoryError> {
    verify_review(plan, plan.plan_sha256)?;
    let policy_text = input_text(policy_bytes)?;
    let seed_text = input_text(seed_bytes)?;
    let mut source: FleetSource = toml::from_str(policy_text)?;
    let mut seed: EstateSeed = toml::from_str(seed_text)?;
    let mut policy_document: toml::Value = toml::from_str(policy_text)?;
    let mut seed_document: toml::Value = toml::from_str(seed_text)?;
    let authority = &plan.authority;
    let mut recovery = authority
        .recovery_controllers
        .iter()
        .map(Principal::to_text)
        .collect::<Vec<_>>();
    recovery.sort();
    let configured_recovery =
        canonical_recovery_controllers(&source.recovery_controllers, authority.operator)?;
    let estate_matches = seed.schema_version == 1
        && !seed.fresh_estate
        && seed.fleet_id == authority.fleet.fleet.fleet_id
        && seed.coordinator == authority.coordinator.to_text();
    let policy_matches = source.schema_version == 1
        && source.operator == authority.operator.to_text()
        && configured_recovery == recovery;
    if !estate_matches || !policy_matches {
        return Err(CapacityImportInventoryError::AuthorityMismatch);
    }
    validate_identity_seed(&source, &seed)?;
    let subnet = authority.subnet.into_principal().to_text();
    let seed_index = seed
        .roots
        .iter()
        .position(|root| root.root == authority.root.to_text() && root.placement_subnet == subnet)
        .ok_or(CapacityImportInventoryError::AuthorityMismatch)?;
    let source_index = source
        .fleet_subnet_roots
        .iter()
        .position(|root| root.placement_subnet == subnet)
        .ok_or(CapacityImportInventoryError::AuthorityMismatch)?;
    let imports = &mut seed.roots[seed_index].pool_imports;
    match kind {
        CapacityImportPublicationKind::ExtendEstate => {
            for candidate in &plan.sources {
                let id = candidate.binding.canister_id.to_text();
                if imports.contains(&id) {
                    return Err(CapacityImportInventoryError::AlreadyPresent);
                }
                imports.push(id);
            }
        }
        CapacityImportPublicationKind::InitializeEstate => {
            let expected = plan
                .sources
                .iter()
                .map(|source| source.binding.canister_id.to_text())
                .collect::<std::collections::BTreeSet<_>>();
            let supplied = imports
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            if imports.len() != supplied.len() || supplied != expected {
                return Err(CapacityImportInventoryError::AuthorityMismatch);
            }
        }
    }
    source.fleet_subnet_roots[source_index]
        .canister_pool
        .imports
        .clone_from(imports);
    let replacement_imports =
        toml::Value::Array(imports.iter().cloned().map(toml::Value::String).collect());
    // Reuse the generator's global identity and capacity rules: candidates may
    // not alias any infrastructure, treasury, or another Root's existing pool.
    validate_identity_seed(&source, &seed)?;
    let source_root = array_entry(&mut policy_document, "fleet_subnet_roots", source_index)?;
    source_root
        .get_mut("canister_pool")
        .and_then(toml::Value::as_table_mut)
        .ok_or(CapacityImportInventoryError::AuthorityMismatch)?
        .insert("imports".to_string(), replacement_imports.clone());
    array_entry(&mut seed_document, "roots", seed_index)?
        .as_table_mut()
        .ok_or(CapacityImportInventoryError::AuthorityMismatch)?
        .insert("pool_imports".to_string(), replacement_imports);
    Ok(CapacityImportInventoryView {
        plan_sha256: plan.plan_sha256,
        policy: replacement(policy_bytes, &policy_document)?,
        seed: replacement(seed_bytes, &seed_document)?,
    })
}

fn input_text(bytes: &[u8]) -> Result<&str, CapacityImportInventoryError> {
    if bytes.len() > MAX_GENERATOR_INPUT_BYTES {
        return Err(CapacityImportInventoryError::TooLarge);
    }
    Ok(std::str::from_utf8(bytes)?)
}

fn array_entry<'a>(
    document: &'a mut toml::Value,
    name: &str,
    index: usize,
) -> Result<&'a mut toml::Value, CapacityImportInventoryError> {
    document
        .get_mut(name)
        .and_then(toml::Value::as_array_mut)
        .and_then(|entries| entries.get_mut(index))
        .ok_or(CapacityImportInventoryError::AuthorityMismatch)
}

fn replacement(
    original: &[u8],
    document: &toml::Value,
) -> Result<CapacityImportDocumentView, CapacityImportInventoryError> {
    let replacement = if document == &toml::from_str::<toml::Value>(input_text(original)?)? {
        original.to_vec()
    } else {
        toml::to_string_pretty(document)?.into_bytes()
    };
    if replacement.len() > MAX_GENERATOR_INPUT_BYTES {
        return Err(CapacityImportInventoryError::TooLarge);
    }
    Ok(CapacityImportDocumentView {
        before_sha256: Sha256::digest(original).into(),
        after_sha256: Sha256::digest(&replacement).into(),
        replacement,
    })
}
