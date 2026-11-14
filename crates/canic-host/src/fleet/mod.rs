use crate::{Error, Network, Plan, Receipt, Result, artifact, network, operation, store, supplier};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path, time::Duration};

#[cfg(test)]
mod tests;

const MAX_OPERATIONS: usize = 128;

/// Read-only observations of the deployments known to one local journal.
#[derive(Debug, Serialize)]
pub struct Fleet {
    /// UTC Unix nanoseconds when inventory collection began; observations are not atomic.
    pub started_at_ns: u64,
    /// UTC Unix nanoseconds when this snapshot finished.
    pub finished_at_ns: u64,
    /// Explicit inventory scope; this is not a network-wide discovery result.
    pub scope: &'static str,
    /// Known targets, deduplicated by exact network trust binding and principal.
    pub canisters: Vec<FleetCanister>,
    /// Retained operations, including plans without a known target.
    pub operations: Vec<FleetOperation>,
    /// Local records that could not be admitted. Other records remain visible.
    pub issues: Vec<String>,
    /// Explicitly selected profile-manager reports, each an uncertified query.
    pub managers: Vec<FleetManager>,
    /// Optional, explicitly selected mainnet default wallet; separate from canister balances.
    pub wallet: Option<crate::Wallet>,
}

/// Application-specific demo adapter; reports do not grant management authority.
#[derive(Debug, Serialize)]
pub struct FleetManager {
    /// Exact network binding from the manager's local deployment plan.
    pub network_id: String,
    /// Known manager principal.
    pub manager: String,
    /// Uncertified operation projection; child metadata is independently observed.
    pub reported: Option<canic_profile_protocol::growth::Status>,
    /// Observation failure, never interpreted as an empty slot.
    pub observation_error: Option<String>,
}

/// One admitted operation's public observation projection, excluding signed requests.
#[derive(Debug, Serialize)]
pub struct FleetOperation {
    /// Digest of the original approved plan.
    pub operation: String,
    /// Digest of the exact network trust binding.
    pub network_id: String,
    /// Human-readable network label.
    pub network: String,
    /// Known principal, or no target while creation remains unresolved.
    pub canister: Option<String>,
    /// Approved module hash, not a claim about the current installation.
    pub approved_module: String,
    /// Approved sole controller, not a current controller observation.
    pub approved_controller: String,
    /// Exact local effect evidence classes from the status command.
    pub effects: BTreeMap<String, String>,
    /// Whether an internally consistent terminal receipt has been retained.
    pub receipt_retained: bool,
    /// Whether this plan's original admission deadline has passed.
    pub approval_expired: bool,
}

/// One canister's current certified management metadata, or an explicit unknown observation.
#[derive(Debug, Serialize)]
pub struct FleetCanister {
    /// Exact network trust binding digest; local replicas never merge by principal alone.
    pub network_id: String,
    /// Human-readable network label.
    pub network: String,
    /// Canister principal.
    pub canister: String,
    /// UTC Unix nanoseconds at the end of this observation attempt.
    pub observed_at_ns: u64,
    /// Certified module hash when both metadata reads succeed.
    pub module_hash: Option<String>,
    /// Certified controllers when both metadata reads succeed; absent means unknown.
    pub controllers: Option<Vec<String>>,
    /// Observation failure or budget exhaustion, never evidence of canister failure.
    pub observation_error: Option<String>,
}

fn label(network: &Network) -> String {
    match network {
        Network::Mainnet => "ic / mainnet".into(),
        Network::Local { url, .. } => format!("local / {url}"),
    }
}

fn local_operation(root: &Path, id: &str, now: u64) -> Result<(FleetOperation, Network)> {
    let state = operation::status(root, id)?;
    let plan: Plan = serde_json::from_value(state["plan"].clone()).map_err(supplier)?;
    let receipt: Option<Receipt> =
        serde_json::from_value(state["receipt"].clone()).map_err(supplier)?;
    let dir = root.join(id);
    let canister = if let Some(target) = &plan.target {
        Some(target.clone())
    } else if dir
        .join("create/reply.bin")
        .try_exists()
        .map_err(supplier)?
    {
        match operation::created(
            &plan,
            &store::read(&dir.join("create/reply.bin"), 2 * 1024 * 1024)?,
        ) {
            Ok(principal) => Some(principal.to_text()),
            Err(Error::Blocked(_)) => None,
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    if let Some(receipt) = &receipt
        && (receipt.operation != id
            || Some(&receipt.canister) != canister.as_ref()
            || receipt.module_hash != plan.artifact.sha256
            || receipt.controller != plan.controller)
    {
        return Err(Error::Invalid(
            "receipt does not match retained plan and target evidence".into(),
        ));
    }
    let network_id = artifact::digest(&serde_json::to_vec(&plan.network).map_err(supplier)?);
    Ok((
        FleetOperation {
            operation: id.into(),
            network_id,
            network: label(&plan.network),
            canister,
            approved_module: plan.artifact.sha256,
            approved_controller: plan.controller,
            effects: serde_json::from_value(state["effects"].clone()).map_err(supplier)?,
            receipt_retained: receipt.is_some(),
            approval_expired: now >= plan.expires_at_ns,
        },
        plan.network,
    ))
}

async fn observe(network: &Network, canister: &str) -> Result<(String, Vec<String>)> {
    let agent = network.agent()?;
    network::principal(canister)?;
    let principal = canister.parse().map_err(supplier)?;
    let module = network::bounded(agent.read_state_canister_info(principal, "module_hash")).await?;
    let controllers =
        network::bounded(agent.read_state_canister_info(principal, "controllers")).await?;
    let controllers =
        ic_platform::ic_protocol::decode_controllers(&controllers, 4096).map_err(supplier)?;
    if module.len() != 32 {
        return Err(Error::Invalid(
            "certified module hash is not 32 bytes".into(),
        ));
    }
    Ok((
        crate::hex(&module),
        controllers.iter().map(|value| value.to_text()).collect(),
    ))
}

/// Observe locally retained deployments using anonymous certified metadata reads.
///
/// No signer, update call, reconciliation or retry is invoked. Each target has a
/// five-second observation limit inside a thirty-second total network budget.
/// Local records are sampled separately; this is neither an atomic snapshot nor
/// complete Fleet discovery. Application readiness and cycles are not observed.
///
/// # Errors
/// Refuses unreadable inventory or more than 128 operation directories. Individual
/// corrupt records and unavailable targets are retained as explicit partial results.
pub async fn fleet(root: &Path) -> Result<Fleet> {
    fleet_with_managers(root, &[]).await
}

/// Include up to sixteen explicit profile managers and their reported workers.
///
/// Managers must be unambiguous known deployments. Public status is uncertified;
/// worker module/controller metadata is independently read through certified state.
/// No update, paid action, reconciliation or further child discovery occurs.
///
/// # Errors
/// Refuses invalid/ambiguous manager selection or unreadable/oversized inventory.
pub async fn fleet_with_managers(root: &Path, selected: &[String]) -> Result<Fleet> {
    if selected.len() > 16 {
        return Err(Error::Invalid(
            "at most sixteen managers per snapshot".into(),
        ));
    }
    let started_at_ns = crate::now()?;
    let mut ids = Vec::new();
    let mut issues = Vec::new();
    for entry in std::fs::read_dir(root).map_err(supplier)? {
        let entry = entry.map_err(supplier)?;
        let name = entry.file_name();
        let Some(id) = name.to_str() else { continue };
        if id.len() != 64
            || !id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            continue;
        }
        if !entry.file_type().map_err(supplier)?.is_dir() {
            issues.push(format!("{id}: operation path is not a regular directory"));
        } else {
            ids.push(id.to_owned());
        }
        if ids.len() + issues.len() > MAX_OPERATIONS {
            return Err(Error::Invalid(
                "fleet inventory exceeds 128 operations; select a smaller state directory".into(),
            ));
        }
    }
    ids.sort();
    let mut operations = Vec::new();
    let mut targets = BTreeMap::new();
    for id in ids {
        match local_operation(root, &id, started_at_ns) {
            Ok((operation, network)) => {
                if let Some(canister) = &operation.canister {
                    targets.insert((operation.network_id.clone(), canister.clone()), network);
                }
                operations.push(operation);
            }
            Err(error) => issues.push(format!("{id}: {error}")),
        }
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut managers = Vec::new();
    for manager in selected {
        network::principal(manager)?;
        let matches: Vec<_> = targets
            .iter()
            .filter(|((_, target), _)| target == manager)
            .collect();
        if matches.len() != 1 {
            return Err(Error::Invalid(
                "manager must be a known target on one unambiguous network".into(),
            ));
        }
        let ((network_id, _), network) = matches[0];
        let network_id = network_id.clone();
        let network = network.clone();
        let budget = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_secs(5));
        let report = tokio::time::timeout(budget, async {
            let agent = network.agent()?;
            let bytes = ic_wire::native::encode_request(
                canic_profile_protocol::growth::STATUS,
                &(),
                canic_profile_protocol::growth::LIMITS,
            )
            .map_err(supplier)?;
            let response = agent
                .query(&manager.parse().map_err(supplier)?, "status")
                .with_arg(bytes)
                .call()
                .await
                .map_err(supplier)?;
            let report: canic_profile_protocol::growth::Status = ic_wire::native::decode_response(
                canic_profile_protocol::growth::STATUS,
                &response,
                canic_profile_protocol::growth::LIMITS,
            )
            .map_err(supplier)?;
            if report
                .approval
                .as_ref()
                .is_some_and(|a| a.manager != *manager)
            {
                return Err(Error::Invalid(
                    "reported approval names another manager".into(),
                ));
            }
            if let Some(child) = &report.worker {
                network::principal(child)?;
            }
            Ok::<_, Error>(report)
        })
        .await
        .map_err(|_| Error::Blocked("manager observation timed out".into()))
        .and_then(|value| value);
        let (reported, observation_error) = match report {
            Ok(report) => {
                if let Some(child) = &report.worker {
                    targets.insert((network_id.clone(), child.clone()), network);
                }
                (Some(report), None)
            }
            Err(error) => (None, Some(error.to_string().chars().take(500).collect())),
        };
        managers.push(FleetManager {
            network_id,
            manager: manager.clone(),
            reported,
            observation_error,
        });
    }
    let mut canisters = Vec::new();
    for ((network_id, canister), network) in targets {
        let budget = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_secs(5));
        let result = if budget.is_zero() {
            Err(Error::Blocked(
                "snapshot network budget exhausted; target not observed".into(),
            ))
        } else {
            tokio::time::timeout(budget, observe(&network, &canister))
                .await
                .map_err(|_| {
                    Error::Blocked("observation timed out; current state is unknown".into())
                })
                .and_then(|value| value)
        };
        let (module_hash, controllers, observation_error) = match result {
            Ok((module, controllers)) => (Some(module), Some(controllers), None),
            Err(error) => (
                None,
                None,
                Some(error.to_string().chars().take(500).collect()),
            ),
        };
        canisters.push(FleetCanister {
            network_id,
            network: label(&network),
            canister,
            observed_at_ns: crate::now()?,
            module_hash,
            controllers,
            observation_error,
        });
    }
    Ok(Fleet {
        started_at_ns,
        finished_at_ns: crate::now()?,
        scope: "local deployment journal only",
        canisters,
        operations,
        issues,
        managers,
        wallet: None,
    })
}
