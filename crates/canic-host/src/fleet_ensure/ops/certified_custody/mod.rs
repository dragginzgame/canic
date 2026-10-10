//! Module: fleet_ensure::ops::certified_custody
//!
//! Responsibility: observe certified controllers, module presence and subnet placement.
//! Boundary: certificate samples carry no payment, balance or reset authority.

#[cfg(test)]
mod tests;

use crate::fleet_ensure::view::certified_custody::CertifiedCanisterCustodyView;
use candid::Principal;
use canic_contracts::ids::SubnetId;
use canic_core::cdk::utils::hash::hex_bytes;
use ic_agent::{Agent, AgentError, Certificate};
use ic_certification::{Label, LookupResult};
use std::{collections::BTreeSet, time::Duration};
use thiserror::Error;

const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// A certificate cannot establish the selected canister's physical custody.
#[derive(Debug, Error)]
pub enum CertifiedCustodyError {
    #[error("certified custody observation of {canister} failed: {source}")]
    Certificate {
        canister: Principal,
        #[source]
        source: Box<AgentError>,
    },
    #[error("certified custody observation of {canister} exceeded its deadline")]
    Deadline { canister: Principal },
    #[error("certificate for {canister} does not prove its controllers and module presence")]
    IncompleteCertificate { canister: Principal },
}

/// Read one certified physical identity using the same decoder as the estate survey.
pub(in crate::fleet_ensure) async fn observe_one(
    agent: &Agent,
    principal: Principal,
) -> Result<CertifiedCanisterCustodyView, CertifiedCustodyError> {
    let certificate = tokio::time::timeout(
        READ_TIMEOUT,
        agent.read_state_raw(paths(principal), principal),
    )
    .await
    .map_err(|_| CertifiedCustodyError::Deadline {
        canister: principal,
    })?
    .map_err(|source| CertifiedCustodyError::Certificate {
        canister: principal,
        source: Box::new(source),
    })?;
    project(&certificate, &agent.read_root_key(), principal)
}

fn paths(principal: Principal) -> Vec<Vec<Label>> {
    ["controllers", "module_hash"]
        .into_iter()
        .map(|field| {
            vec![
                "canister".into(),
                Label::from_bytes(principal.as_slice()),
                field.into(),
            ]
        })
        .collect()
}

/// Decode only after Agent has verified the certificate and its delegation for this target.
fn project(
    certificate: &Certificate,
    root_key: &[u8],
    principal: Principal,
) -> Result<CertifiedCanisterCustodyView, CertifiedCustodyError> {
    let invalid = || CertifiedCustodyError::IncompleteCertificate {
        canister: principal,
    };
    let path = |field: &'static [u8]| [b"canister".as_slice(), principal.as_slice(), field];
    let bytes = ic_agent::lookup_value(certificate, path(b"controllers")).map_err(|_| invalid())?;
    let mut input = bytes;
    let mut controllers: Vec<Principal> =
        ciborium::de::from_reader_with_recursion_limit(&mut input, 8).map_err(|_| invalid())?;
    let unique = controllers.iter().collect::<BTreeSet<_>>();
    if !input.is_empty()
        || controllers.is_empty()
        || controllers.len() > 10
        || unique.len() != controllers.len()
    {
        return Err(invalid());
    }
    controllers.sort_unstable();
    let module_sha256 = match certificate.tree.lookup_path(path(b"module_hash")) {
        LookupResult::Found(bytes) if bytes.len() == 32 => Some(hex_bytes(bytes)),
        LookupResult::Absent => None,
        _ => return Err(invalid()),
    };
    let subnet = match certificate.delegation.as_ref() {
        Some(delegation) => {
            Principal::try_from_slice(&delegation.subnet_id).map_err(|_| invalid())?
        }
        None => Principal::self_authenticating(root_key),
    };
    Ok(CertifiedCanisterCustodyView {
        principal,
        subnet: SubnetId::from_principal(subnet),
        controllers,
        module_sha256,
        certificate_tree_sha256: certificate.tree.digest(),
    })
}
