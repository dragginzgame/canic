use crate::{Error, Network, Plan, Receipt, Result, network, operation, store, supplier};
use serde::Serialize;
use std::{collections::BTreeMap, path::Path, time::Duration};

#[cfg(test)]
mod tests;

const MAX_OPERATIONS: usize = 128;

/// Read-only observations selected by a local journal and optional configuration.
#[derive(Debug, Serialize)]
pub struct Fleet {
    /// Optional local declaration; separate from observed deployment evidence.
    pub configuration: Option<crate::SelectedFleet>,
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
    /// Planned Roots and explicitly selected managers, each an uncertified query.
    pub managers: Vec<FleetManager>,
    /// Optional, explicitly selected mainnet default wallet; separate from canister balances.
    pub wallet: Option<crate::Wallet>,
}

/// Management protocol observation; reports do not grant authority or output custody.
#[derive(Debug, Serialize)]
pub struct FleetManager {
    /// Exact network binding from the manager's local plan or configuration.
    pub network_id: String,
    /// Known manager principal.
    pub manager: String,
    /// Uncertified operation projection; child metadata is independently observed.
    pub reported: Option<canic_management_protocol::Status>,
    /// Observation failure, never interpreted as an empty slot.
    pub observation_error: Option<String>,
}

/// One admitted operation's public observation projection, excluding signed requests.
#[derive(Debug, Serialize)]
pub struct FleetOperation {
    /// Exact planned Root-owned work, used only to select and check read-only reports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub management: Option<canic_management_protocol::Approval>,
    /// Names committed in the approved plan, if created through a fleet configuration.
    pub deployment: Option<crate::DeploymentName>,
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

pub(crate) fn local_operation(
    root: &Path,
    id: &str,
    now: u64,
) -> Result<(FleetOperation, Network)> {
    let state = operation::status(root, id)?;
    let plan: Plan = serde_json::from_value(state["plan"].clone()).map_err(supplier)?;
    let receipt: Option<Receipt> =
        serde_json::from_value(state["receipt"].clone()).map_err(supplier)?;
    let dir = root.join(id);
    let canister = if plan.management.is_some() {
        crate::managed::evidence(&dir, &plan)?.and_then(|status| status.worker)
    } else if let Some(target) = &plan.target {
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
            || receipt.controller != plan.application_controller())
    {
        return Err(Error::Invalid(
            "receipt does not match retained plan and target evidence".into(),
        ));
    }
    let network_id = plan.network.binding_id()?;
    let approved_controller = plan.application_controller().to_owned();
    Ok((
        FleetOperation {
            management: plan
                .management
                .as_ref()
                .map(|_| crate::managed::approval(&plan))
                .transpose()?,
            deployment: plan.deployment,
            operation: id.into(),
            network_id,
            network: label(&plan.network),
            canister,
            approved_module: plan.artifact.sha256,
            approved_controller,
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

fn select_managers(
    operations: &[FleetOperation],
    targets: &BTreeMap<(String, String), Network>,
    explicit: &std::collections::BTreeSet<String>,
) -> Result<BTreeMap<(String, String), Network>> {
    let mut selected = BTreeMap::new();
    for operation in operations {
        if let Some(approval) = &operation.management {
            let key = (operation.network_id.clone(), approval.manager.clone());
            let network = targets
                .get(&key)
                .ok_or_else(|| Error::Invalid("planned Root has no network binding".into()))?;
            selected.insert(key, network.clone());
        }
    }
    // Explicit principal-only selection still requires one unambiguous network.
    // Automatic selection carries the exact network from each admitted plan.
    for principal in explicit {
        network::principal(principal)?;
        let matches: Vec<_> = targets
            .iter()
            .filter(|((_, target), _)| target == principal)
            .collect();
        let [(key, network)] = matches.as_slice() else {
            return Err(Error::Invalid(
                "manager must be a known target on one unambiguous network".into(),
            ));
        };
        selected.insert((*key).clone(), (*network).clone());
    }
    if selected.len() > 16 {
        return Err(Error::Invalid(
            "at most sixteen network-bound managers per snapshot".into(),
        ));
    }
    Ok(selected)
}

fn check_report(
    operations: &[FleetOperation],
    network_id: &str,
    manager: &str,
    report: &canic_management_protocol::Status,
) -> Result<()> {
    if report
        .approval
        .as_ref()
        .is_some_and(|approval| approval.manager != manager)
    {
        return Err(Error::Invalid(
            "reported approval names another manager".into(),
        ));
    }
    let empty = report
        == &canic_management_protocol::Status {
            stable_pages: report.stable_pages,
            maintenance_candidate: report.maintenance_candidate,
            ..Default::default()
        };
    if !empty
        && operations
            .iter()
            .filter(|operation| operation.network_id == network_id)
            .filter_map(|operation| operation.management.as_ref())
            .filter(|approval| approval.manager == manager)
            .any(|approval| report.approval.as_ref() != Some(approval))
    {
        return Err(Error::Invalid("Root report differs from retained child approval; reported child not admitted to inventory".into()));
    }
    if let Some(child) = &report.worker {
        let child = network::principal(child)?;
        if child == network::principal(manager)?
            || child == network::principal("aaaaa-aa")?
            || child == network::principal("2vxsx-fae")?
        {
            return Err(Error::Invalid(
                "reported child is not a distinct canister".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) struct Inventory {
    pub records: Vec<(FleetOperation, Network)>,
    pub issues: Vec<String>,
}

pub(crate) fn inventory(
    root: &Path,
    missing_allowed: bool,
    started_at_ns: u64,
) -> Result<Inventory> {
    let mut ids = Vec::new();
    let mut issues = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => Some(entries),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && missing_allowed => None,
        Err(error) => return Err(supplier(error)),
    };
    for entry in entries.into_iter().flatten() {
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
    let mut records = Vec::new();
    for id in ids {
        match local_operation(root, &id, started_at_ns) {
            Ok(record) => records.push(record),
            Err(error) => issues.push(format!("{id}: {error}")),
        }
    }
    Ok(Inventory { records, issues })
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

/// Include planned Roots and explicit managers, at most sixteen network-bound owners.
///
/// Managers must be unambiguous known deployments. Public status is uncertified;
/// worker module/controller metadata is independently read through certified state.
/// No update, paid action, reconciliation or further child discovery occurs.
///
/// # Errors
/// Refuses invalid/ambiguous manager selection or unreadable/oversized inventory.
pub async fn fleet_with_managers(root: &Path, selected: &[String]) -> Result<Fleet> {
    fleet_with_config(root, selected, None).await
}

/// Observe a configured fleet, including undeployed named slots in its declaration.
///
/// # Errors
/// Refuses invalid configuration and the same inventory/selection failures as [`fleet_with_managers`].
pub async fn fleet_with_config(
    root: &Path,
    selected: &[String],
    configuration: Option<&crate::SelectedFleet>,
) -> Result<Fleet> {
    if let Some(config) = configuration {
        config.validate()?;
    }
    let started_at_ns = crate::now()?;
    let Inventory {
        records,
        mut issues,
    } = inventory(root, configuration.is_some(), started_at_ns)?;
    let known = configuration
        .map(|config| config.known_targets(&records))
        .unwrap_or_default();
    let mut operations = Vec::new();
    let mut targets = BTreeMap::new();
    for (operation, network) in records {
        let id = &operation.operation;
        if let Some(config) = configuration {
            if !config.contains(&operation, &network, &known) {
                if operation.deployment.is_none() && operation.canister.is_none() {
                    issues.push(format!("{id}: unassigned legacy operation remains in this journal; inspect with --no-config status {id}"));
                }
                continue;
            }
            if config.config.network != network {
                issues.push(format!("{id}: retained deployment network differs from fleet configuration; original evidence is preserved"));
            }
        }
        if let Some(approval) = &operation.management {
            targets.insert(
                (operation.network_id.clone(), approval.manager.clone()),
                network.clone(),
            );
        }
        if let Some(canister) = &operation.canister {
            targets.insert((operation.network_id.clone(), canister.clone()), network);
        }
        operations.push(operation);
    }
    let mut selected: std::collections::BTreeSet<_> = selected.iter().cloned().collect();
    if let Some(config) = configuration {
        let network_id = config.config.network.binding_id()?;
        for (slot, declaration) in &config.config.canisters {
            if let Some(id) = &declaration.id {
                targets.insert(
                    (network_id.clone(), id.clone()),
                    config.config.network.clone(),
                );
                if declaration.profile_manager {
                    selected.insert(id.clone());
                }
                if operations.iter().any(|operation| {
                    operation.deployment.as_ref().is_some_and(|name| {
                        name.fleet == config.fleet
                            && name.app == config.app
                            && name.canister == *slot
                    }) && (operation.network_id != network_id
                        || operation.canister.as_ref() != Some(id))
                }) {
                    issues.push(format!("{slot}: configured identity conflicts with a retained named operation; original evidence is preserved"));
                }
            }
        }
    }
    let selected = select_managers(&operations, &targets, &selected)?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut managers = Vec::new();
    for ((network_id, manager), network) in selected {
        let budget = deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_secs(5));
        let report = tokio::time::timeout(budget, async {
            let agent = network.agent()?;
            let bytes = ic_wire::native::encode_request(
                canic_management_protocol::STATUS,
                &(),
                canic_management_protocol::LIMITS,
            )
            .map_err(supplier)?;
            let response = agent
                .query(&manager.parse().map_err(supplier)?, "status")
                .with_arg(bytes)
                .call()
                .await
                .map_err(supplier)?;
            let report: canic_management_protocol::Status = ic_wire::native::decode_response(
                canic_management_protocol::STATUS,
                &response,
                canic_management_protocol::LIMITS,
            )
            .map_err(supplier)?;
            check_report(&operations, &network_id, &manager, &report)?;
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
            manager,
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
        configuration: configuration.cloned(),
        started_at_ns,
        finished_at_ns: crate::now()?,
        scope: "local deployment journal, configured identities, planned Roots and selected manager reports",
        canisters,
        operations,
        issues,
        managers,
        wallet: None,
    })
}
