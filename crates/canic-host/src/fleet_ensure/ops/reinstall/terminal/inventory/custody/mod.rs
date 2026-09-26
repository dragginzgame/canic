//! Certified physical custody observations for completed source evidence.
//!
//! Reads IC certificates only. No source runtime method, management update, paid
//! inspection, controller change or current-authority publication occurs here.
//! A sample is not a mutation fence, complete live inventory or balance admission.

#[cfg(test)]
mod tests;

use crate::{
    fleet_ensure::{
        CompletedEstateInventoryView,
        model::{
            DesiredCanisterKind, FleetTerminalSourceRecord,
            completed_handoff::{
                CompletedCanisterCustodyRecord, CompletedEstateCustodyRecord,
                CompletedPhysicalBindingRecord,
            },
        },
        ops::retained_contract::{RetainedContractError, inspect_completed_source},
        view::terminal_source::inventory::{
            CompletedCanisterCustodyView, CompletedEstateCustodyView,
        },
    },
    icp::{IcpCli, IcpManagementCallError},
};
use candid::Principal;
use canic_core::{
    cdk::utils::hash::hex_bytes,
    ids::{CanonicalNetworkId, SubnetId},
};
use ic_agent::{Agent, AgentError, Certificate};
use ic_certification::{Label, LookupResult};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Duration,
};
use thiserror::Error;

const RESPONSE_BYTES: usize = 256 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(30);
const SAMPLE_LIFETIME: Duration = Duration::from_secs(60);

///
/// CompletedCustodyError
///
/// Failure to bind a fresh certified observation to exact source membership.
/// No error authorizes a replacement ID or a destructive effect.
///
#[derive(Debug, Error)]
pub enum CompletedCustodyError {
    #[error(
        "certified custody samples expired; inspect the exact estate again before review or apply"
    )]
    Expired,
    #[error("completed source inspection failed: {0}")]
    Source(#[from] Box<RetainedContractError>),
    #[error("completed source changed during custody inspection")]
    SourceChanged,
    #[error("source custody signer/network setup failed: {0}")]
    Identity(#[source] Box<IcpManagementCallError>),
    #[error("selected signer or network differs from completed source evidence")]
    ReaderMismatch,
    #[error("invalid completed source physical membership or owner binding")]
    Membership,
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
    #[error("canister {canister} is outside its recorded subnet")]
    SubnetMismatch { canister: Principal },
    #[error("canister {canister} no longer has its recorded installed module")]
    ModuleMismatch { canister: Principal },
    #[error("canister {canister} is not controlled by required source owner {owner}")]
    ControllerMismatch {
        canister: Principal,
        owner: Principal,
    },
}

/// Observe supplied source IDs using authenticated certified state, without canister execution.
///
/// Controller lists are observed facts, never defaults derived from historical configuration.
/// All balances, complete membership and a journaled mutation fence remain separate admission.
pub async fn inspect(
    workspace: &Path,
    environment: &str,
    fleet: &str,
    icp: &IcpCli,
) -> Result<CompletedEstateCustodyView, CompletedCustodyError> {
    let source = inspect_completed_source(workspace, environment, fleet).map_err(Box::new)?;
    let agent = icp
        .authenticated_agent_with_response_limit(RESPONSE_BYTES)
        .map_err(|error| CompletedCustodyError::Identity(Box::new(error)))?;
    let view = observe(&agent, &source.inventory).await?;
    let refreshed = inspect_completed_source(workspace, environment, fleet).map_err(Box::new)?;
    if refreshed.inventory.receipts.documents != view.documents
        || refreshed.source_protocols != source.source_protocols
    {
        return Err(CompletedCustodyError::SourceChanged);
    }
    Ok(view)
}

pub(super) async fn observe(
    agent: &Agent,
    source: &CompletedEstateInventoryView,
) -> Result<CompletedEstateCustodyView, CompletedCustodyError> {
    let observed_at = std::time::Instant::now();
    let operator = Principal::from_text(&source.receipts.source_operator)
        .map_err(|_| CompletedCustodyError::ReaderMismatch)?;
    let root_key = agent.read_root_key();
    let network = CanonicalNetworkId::from_der_root_trust_anchor(&root_key)
        .map_err(|_| CompletedCustodyError::ReaderMismatch)?;
    if agent.get_principal().ok() != Some(operator)
        || network != source.fleet.fleet.canonical_network_id
    {
        return Err(CompletedCustodyError::ReaderMismatch);
    }
    let owners = owners(source, operator)?;
    let mut canisters = BTreeMap::new();
    // Each response proves controllers, module presence and delegated subnet together.
    // Samples across different canisters do not promise an atomic estate snapshot.
    for (name, recorded) in &source.canisters {
        if observed_at.elapsed() > SAMPLE_LIFETIME {
            return Err(CompletedCustodyError::Expired);
        }
        let principal = recorded.principal;
        let view = observe_one(agent, principal).await?;
        if view.subnet != recorded.subnet {
            return Err(CompletedCustodyError::SubnetMismatch {
                canister: principal,
            });
        }
        if view.module_sha256 != recorded.module_sha256 {
            return Err(CompletedCustodyError::ModuleMismatch {
                canister: principal,
            });
        }
        let owner = owners[name];
        if !view.controllers.contains(&owner) {
            return Err(CompletedCustodyError::ControllerMismatch {
                canister: principal,
                owner,
            });
        }
        canisters.insert(name.clone(), view);
    }
    let view = CompletedEstateCustodyView {
        observed_at,
        documents: source.receipts.documents.clone(),
        network,
        operator,
        canisters,
    };
    capture(&view, &source.receipts.documents)?;
    Ok(view)
}

/// Read one certified physical identity using the same decoder as the estate survey.
pub(in crate::fleet_ensure) async fn observe_one(
    agent: &Agent,
    principal: Principal,
) -> Result<CompletedCanisterCustodyView, CompletedCustodyError> {
    let certificate = tokio::time::timeout(
        READ_TIMEOUT,
        agent.read_state_raw(paths(principal), principal),
    )
    .await
    .map_err(|_| CompletedCustodyError::Deadline {
        canister: principal,
    })?
    .map_err(|source| CompletedCustodyError::Certificate {
        canister: principal,
        source: Box::new(source),
    })?;
    project(&certificate, &agent.read_root_key(), principal)
}

/// Convert fresh authenticated samples for review without carrying executable source state.
pub(in crate::fleet_ensure) fn capture(
    view: &CompletedEstateCustodyView,
    source: &FleetTerminalSourceRecord,
) -> Result<CompletedEstateCustodyRecord, CompletedCustodyError> {
    if view.observed_at.elapsed() > SAMPLE_LIFETIME {
        return Err(CompletedCustodyError::Expired);
    }
    if view.documents != *source {
        return Err(CompletedCustodyError::SourceChanged);
    }
    Ok(CompletedEstateCustodyRecord {
        network: view.network,
        operator: view.operator,
        canisters: view
            .canisters
            .iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    CompletedCanisterCustodyRecord {
                        binding: CompletedPhysicalBindingRecord {
                            principal: entry.principal,
                            subnet: entry.subnet,
                            controllers: entry.controllers.clone(),
                            module_sha256: entry.module_sha256.clone(),
                        },
                        certificate_tree_sha256: entry.certificate_tree_sha256,
                    },
                )
            })
            .collect(),
    })
}

/// Fresh certificates have new tree roots. Every selected physical field must still match.
pub(in crate::fleet_ensure) fn same_authority(
    reviewed: &CompletedEstateCustodyRecord,
    fresh: &CompletedEstateCustodyRecord,
) -> bool {
    let expected = ReaderIdentity {
        network: reviewed.network,
        operator: reviewed.operator,
    };
    let actual = ReaderIdentity {
        network: fresh.network,
        operator: fresh.operator,
    };
    expected == actual
        && reviewed.canisters.len() == fresh.canisters.len()
        && reviewed.canisters.iter().all(|(name, expected)| {
            fresh
                .canisters
                .get(name)
                .is_some_and(|actual| actual.binding == expected.binding)
        })
}

#[derive(Eq, PartialEq)]
struct ReaderIdentity {
    network: CanonicalNetworkId,
    operator: Principal,
}

fn owners(
    source: &CompletedEstateInventoryView,
    operator: Principal,
) -> Result<BTreeMap<String, Principal>, CompletedCustodyError> {
    let mut ids = BTreeSet::new();
    let mut owners = BTreeMap::new();
    if source.canisters.is_empty()
        || source.canisters.len() > crate::fleet_ensure::model::MAX_FLEET_ENSURE_CANISTERS
    {
        return Err(CompletedCustodyError::Membership);
    }
    for (name, entry) in &source.canisters {
        if !ids.insert(entry.principal) {
            return Err(CompletedCustodyError::Membership);
        }
        let owner = match entry.kind {
            DesiredCanisterKind::Coordinator | DesiredCanisterKind::Root => operator,
            DesiredCanisterKind::Store
            | DesiredCanisterKind::Pool
            | DesiredCanisterKind::Component => {
                let root = entry
                    .root
                    .as_ref()
                    .and_then(|root| source.canisters.get(root))
                    .filter(|root| {
                        root.kind == DesiredCanisterKind::Root && root.subnet == entry.subnet
                    })
                    .ok_or(CompletedCustodyError::Membership)?;
                root.principal
            }
            DesiredCanisterKind::Auxiliary => return Err(CompletedCustodyError::Membership),
        };
        owners.insert(name.clone(), owner);
    }
    Ok(owners)
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
) -> Result<CompletedCanisterCustodyView, CompletedCustodyError> {
    let invalid = || CompletedCustodyError::IncompleteCertificate {
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
    Ok(CompletedCanisterCustodyView {
        principal,
        subnet: SubnetId::from_principal(subnet),
        controllers,
        module_sha256,
        certificate_tree_sha256: certificate.tree.digest(),
    })
}
