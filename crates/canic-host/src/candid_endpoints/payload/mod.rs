//! Module: candid_endpoints::payload
//!
//! Responsibility: join compiled payload metadata to the declared Candid methods.
//! Boundary: missing metadata is unknown; prose never supplies an enforcement claim.

use super::{CandidEndpointError, EndpointEntry, EndpointMode};
use canic_core::ingress::payload_contract::{CANDID_PAYLOAD_CONTRACT_PREFIX, PayloadContract};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

///
/// EndpointPayloadLimits
///
/// Encoded argument limits and their enforcement scope for one update method.
///

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct EndpointPayloadLimits {
    pub ingress_max_bytes: Option<u64>,
    pub update_guard_max_bytes: Option<u64>,
    pub ingress_basis: IngressPayloadBasis,
}

///
/// IngressPayloadBasis
///
/// Variant-dependent protocol methods cannot be described as one fixed ingress limit.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IngressPayloadBasis {
    ManagedDefault,
    ExplicitOverride,
    VariantDependent,
}

pub(super) fn attach(
    candid: &str,
    endpoints: &mut [EndpointEntry],
) -> Result<(), CandidEndpointError> {
    let invalid = || CandidEndpointError::InvalidPayloadContract;
    let mut records = candid
        .lines()
        .filter_map(|line| line.strip_prefix(CANDID_PAYLOAD_CONTRACT_PREFIX));
    let Some(encoded) = records.next() else {
        return Ok(());
    };
    if records.next().is_some() || encoded.len() > 1024 * 1024 {
        return Err(invalid());
    }
    let bytes = canic_core::cdk::utils::hash::decode_hex(encoded).map_err(|_| invalid())?;
    let mut remaining = bytes.as_slice();
    let contract: PayloadContract =
        ciborium::de::from_reader(&mut remaining).map_err(|_| invalid())?;
    if contract.schema_version != 1 || !remaining.is_empty() {
        return Err(invalid());
    }
    let mut overrides = BTreeMap::new();
    for declared in &contract.update_overrides {
        if overrides
            .insert(declared.method.as_str(), declared.max_bytes)
            .is_some()
        {
            return Err(invalid());
        }
    }
    let variants: BTreeSet<_> = contract
        .variant_dependent_methods
        .iter()
        .map(String::as_str)
        .collect();
    if variants.len() != contract.variant_dependent_methods.len() {
        return Err(invalid());
    }
    for endpoint in endpoints {
        let explicit = overrides.get(endpoint.name.as_str()).copied();
        let variant = variants.contains(endpoint.name.as_str());
        if endpoint
            .modes
            .iter()
            .any(|mode| matches!(mode, EndpointMode::Query | EndpointMode::CompositeQuery))
        {
            if explicit.is_some() || variant {
                return Err(invalid());
            }
            continue;
        }
        endpoint.payload_limits = Some(EndpointPayloadLimits {
            ingress_max_bytes: (!variant)
                .then_some(explicit.unwrap_or(contract.default_update_ingress_max_bytes)),
            update_guard_max_bytes: explicit,
            ingress_basis: if variant {
                IngressPayloadBasis::VariantDependent
            } else if explicit.is_some() {
                IngressPayloadBasis::ExplicitOverride
            } else {
                IngressPayloadBasis::ManagedDefault
            },
        });
    }
    Ok(())
}
