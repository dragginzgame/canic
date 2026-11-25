//! Human projection of retained operation evidence; never a live status check.

#[cfg(test)]
mod tests;

pub(crate) mod management;
pub(crate) mod store;

use canic_host::{
    CanisterStatus, Error, ManagementStatus, Network, OperationStatus, Result, StatusDetail,
};
use ic_host_text::{Alignment, escape, plain_table};
use std::{
    collections::BTreeMap,
    fmt::Write as _,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn plan_preview<'a>(
    plan: &'a canic_host::FleetPlanning,
    journal: &Path,
) -> Result<(Option<&'a str>, String)> {
    use canic_host::FleetPlanning;
    let mut text = match plan {
        FleetPlanning::ShardPool {
            deployment,
            network,
            controller,
            approval,
            capacity,
            increases,
            routes,
            ..
        } => {
            let mut text = pool_header(&deployment.app, &deployment.fleet, network, controller)?;
            pool_terms(&mut text, capacity.as_ref(), None, increases);
            write_shard(&mut text, approval, routes.approval.1);
            text
        }
        FleetPlanning::PlacementPools {
            app,
            fleet,
            network,
            controller,
            capacity,
            standby,
            increases,
            routes,
            shards,
            instances,
            singleton,
            workers,
        } => {
            let mut text = pool_header(app, fleet, network, controller)?;
            pool_terms(
                &mut text,
                capacity.as_ref(),
                standby.as_ref().map(|g| &g.approval),
                increases,
            );
            for shard in shards {
                write_shard(
                    &mut text,
                    &shard.approval,
                    routes[&shard.approval.pool].approval.1,
                );
            }
            if let Some(instance) = instances {
                write_allocation(&mut text, "Registry", &instance.approval);
            }
            if let Some(grant) = singleton {
                let grant = &grant.approval;
                writeln!(
                    text,
                    "\nSingleton · {}\nRegistry: {}\nChild limit: {}",
                    escape(&grant.role),
                    grant.registry,
                    grant.maximum
                )
                .unwrap();
                pool_release(
                    &mut text,
                    &grant.reference,
                    grant.creation_budget,
                    grant.expires_at_ns,
                );
            }
            for worker in workers {
                write_allocation(&mut text, "Scaling workers", &worker.approval);
            }
            text
        }
        FleetPlanning::Create {
            operation,
            fleet_plan,
        } => {
            return Ok((
                Some(operation),
                fleet_preview(operation, fleet_plan, journal)?,
            ));
        }
        FleetPlanning::AddChild { operation, .. }
        | FleetPlanning::Upgrade { operation, .. }
        | FleetPlanning::Maintenance { operation, .. }
        | FleetPlanning::Publication { operation, .. } => {
            return Ok((
                Some(operation),
                render(&canic_host::status(journal, operation)?, journal)?,
            ));
        }
        FleetPlanning::Unchanged { .. } => return Ok((
            None,
            "No artifact changes. Matches retained completion; network state was not refreshed.\n"
                .into(),
        )),
    };
    text.push_str("\nOffline review only. No operation submitted.\n");
    Ok((None, text))
}

fn pool_header(app: &str, fleet: &str, network: &Network, controller: &str) -> Result<String> {
    let location = match network {
        Network::Mainnet => "IC mainnet",
        Network::Local { url, .. } => url,
    };
    Ok(format!(
        "Placement approvals · {}/{}\nNetwork: {}\nNetwork binding: {}\nController: {controller}\n",
        app,
        fleet,
        escape(location),
        network.binding_id()?
    ))
}

fn pool_terms(
    text: &mut String,
    capacity: Option<&canic_host::PoolGrant<canic_management_protocol::stored::SubnetCapacity>>,
    standby: Option<&canic_management_protocol::stored::StandbyPolicy>,
    increases: &[canic_host::PoolGrant<canic_management_protocol::stored::IncreasePoolLimit>],
) {
    if let Some(cap) = capacity {
        writeln!(
            text,
            "Managed subnet ceiling: {} children",
            cap.approval.max_children
        )
        .unwrap();
    }
    if let Some(p) = standby {
        writeln!(text, "Reserve Root: {}", escape(&p.root)).unwrap();
        writeln!(
            text,
            "Min available spares: {} (subnet fleet) · at most {} automatic creations · deadline {} Unix ns",
            p.minimum_available, p.maximum_creations, p.expires_at_ns
        )
        .unwrap();
        for candidate in &p.imports {
            let id = &candidate.principal;
            writeln!(
                text,
                "  Import {id} · minimum {} cycles",
                p.minimum_import_cycles
            )
            .unwrap();
            if let Some(previous) = &candidate.previous_root {
                writeln!(text, "    Previous Root: {}", escape(previous)).unwrap();
            }
        }
    }
    for change in increases {
        use canic_management_protocol::stored::IncreasePoolLimit;
        let (pool, old, new) = match &change.approval {
            IncreasePoolLimit::Shards(a, n) => (&a.pool, a.max_shards, n),
            IncreasePoolLimit::Instances(a, n) => (&a.component, a.max_instances, n),
        };
        writeln!(text, "Increase {}: {old} → {new} children", escape(pool)).unwrap();
    }
}

fn write_shard(text: &mut String, a: &canic_management_protocol::stored::ShardPool, keys: u32) {
    writeln!(text, "\nShared shards · {}\nRoot: {}\nHub: {}\nCreation limit: {} shards\nHub policy: {keys} live keys per shard\nInit bytes (hex): {}", escape(&a.pool), a.root, a.hub, a.max_shards, hex(a.arguments.as_slice())).unwrap();
    pool_release(text, &a.reference, a.creation_budget, a.expires_at_ns);
}

fn write_allocation(
    text: &mut String,
    label: &str,
    a: &canic_management_protocol::stored::InstancePool,
) {
    writeln!(
        text,
        "\n{label} · {}\nRoot: {}\nParent: {}\nChild limit: {}",
        escape(&a.component),
        a.root,
        a.hub,
        a.max_instances
    )
    .unwrap();
    pool_release(text, &a.reference, a.creation_budget, a.expires_at_ns);
}

fn pool_release(
    text: &mut String,
    reference: &canic_management_protocol::stored::Reference,
    budget: u64,
    deadline: u64,
) {
    let m = &reference.manifest;
    writeln!(text, "Store: {}\nRelease: {} @ {}\nWasm SHA-256: {}\nCreation payer: requester\nCreation budget: {budget} cycles\nGrowth deadline: {deadline} Unix ns", reference.store, escape(&m.release.name), escape(&m.release.version), hex(&m.hash)).unwrap();
}

pub(crate) fn render(state: &OperationStatus, journal: &Path) -> Result<String> {
    let state = match state {
        OperationStatus::Fleet(state) => return render_fleet(state, journal),
        OperationStatus::Canister(state) => state,
    };
    let plan = &state.plan;
    if let Some(call) = plan.application_call() {
        return Ok(format!(
            "CANIC APPLICATION CALL\nOperation: {}\nCanister: {}\nCaller: {}\nMethod: {}\nRequest: {} ({} bytes)\nState: {}\nJournal: {}\n",
            state.operation,
            call.canister,
            escape(&plan.controller),
            escape(&call.method),
            plan.artifact.sha256,
            plan.artifact.bytes,
            state.effects.get("call").map_or("unknown", String::as_str),
            escape(&journal.to_string_lossy())
        ));
    }
    let receipt = state.receipt.as_ref();
    let effects = &state.effects;
    let operation = &state.operation;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Supplier(Box::new(error)))?
        .as_nanos();
    let mut output = format!(
        "CANIC OPERATION\nOperation: {}\nJournal: {}\nLocal evidence only; no network observation.\n",
        escape(operation),
        escape(&journal.to_string_lossy()),
    );
    if let Some(name) = &plan.deployment {
        writeln!(
            output,
            "App: {}\nFleet: {}\nCanister slot: {}",
            escape(&name.app),
            escape(&name.fleet),
            escape(&name.canister),
        )
        .expect("writing to String");
    }
    let network = match &plan.network {
        Network::Mainnet => "IC mainnet".into(),
        Network::Local { url, .. } => format!("Local replica: {url}"),
    };
    writeln!(
        output,
        "\nPlanned action: {}\nNetwork: {}\nNetwork binding: {}\nController: {}\nModule: {} ({} bytes)\nCreation debit ceiling: {} cycles\nNew-effect deadline: {} Unix seconds ({})",
        match plan.action { canic_host::PlanAction::Call(_) => unreachable!("call rendered above"), canic_host::PlanAction::Auth(_) => "configure or revoke exact Auth grant (no deployment)", canic_host::PlanAction::Remove(_) => "remove exact Store release (no deployment)", canic_host::PlanAction::UpgradeStore { .. } => "Root upgrades Store, preserving its closed catalog and pins", canic_host::PlanAction::CreateStore { .. } => "create Store, hand control to Root and verify readiness", canic_host::PlanAction::Publish(_) => "publish artifact to Store (no deployment)", canic_host::PlanAction::MaintainRoot {..} => "upgrade Root, preserving child custody", canic_host::PlanAction::UpgradeChild {..} => "Root upgrades its existing child", canic_host::PlanAction::CreateChild(_) => "Root creates and installs its child", canic_host::PlanAction::Upgrade(_) => "upgrade existing canister", canic_host::PlanAction::Create {..} => "create and install canister" },
        escape(&network),
        plan.network.binding_id()?,
        escape(plan.application_controller()),
        escape(&plan.artifact.sha256),
        plan.artifact.bytes,
        plan.creation_cycles(),
        plan.expires_at_ns / 1_000_000_000,
        if now >= u128::from(plan.expires_at_ns) { "expired" } else { "within original deadline" },
    )
    .expect("writing to String");
    if let Some(auth) = plan.auth() {
        writeln!(output,
            "\nAuth action: {}\nRoot: {}\nReceiver: {}\nReceiver controller: {}\nReceiver module: {}\nIssuer: {}\nPublic key: {}\nCanonical network ID: {}\nApplication ID: {}\nApplication role: {}\nScopes: {}\nGrant window: {}–{} Unix ns\nAuth reply: {} (historical; not receiver readiness)",
            if auth.revoke { "revoke" } else { "configure" },
            escape(&auth.root.canister), escape(&auth.receiver.canister), escape(auth.expected_receiver_controller(&plan.controller)), escape(&auth.receiver.module),
            auth.grant.issuer.to_text(),
            hex(&auth.grant.key),
            hex(&auth.grant.audience.canonical_network_id),
            hex(&auth.grant.audience.fleet_id),
            escape(auth.grant.grant.target.as_str()),
            auth.grant.grant.scopes.as_slice().iter().map(|s| escape(s.as_str())).collect::<Vec<_>>().join(", "),
            auth.grant.not_before_ns, auth.grant.expires_at_ns,
            if matches!(&state.detail, StatusDetail::Auth { auth: Some(_) }) { "retained" } else { "unconfirmed" },
        ).expect("String write");
    }
    if let Some(root) = plan.store_root().or_else(|| plan.store_upgrade_root()) {
        writeln!(
            output,
            "Root: {}\nRoot module: {}",
            escape(&root.canister),
            escape(&root.module)
        )
        .expect("String write");
    }
    if plan.store_upgrade_root().is_some() {
        if let StatusDetail::StoreMaintenance {
            store_maintenance: Some(status),
        } = &state.detail
        {
            output.push_str(&store::render(status, Some(now), receipt.is_some()));
        } else {
            output.push_str("Store maintenance: no retained observation\n");
        }
    }
    if let Some(value) = plan.removal() {
        writeln!(
            output,
            "Root: {}\nRoot module: {}\nStore: {}\nStore module: {}\nRelease: {} / {}\nRemoval: {:?}\nSource publication: {}",
            escape(&value.publication.root.canister),
            escape(&value.publication.root.module),
            escape(&value.publication.store.canister),
            escape(&value.publication.store.module),
            escape(&value.publication.release.name),
            escape(&value.publication.release.version),
            value.action,
            escape(&value.publication_operation)
        )
        .expect("String write");
        if let StatusDetail::Removal {
            removal: Some(receipt),
        } = &state.detail
        {
            writeln!(
                output,
                "Removal result: retained\nOriginal dispatch deadline: {} Unix ns",
                receipt.dispatch_deadline_ns
            )
            .expect("String write");
        }
        output.push_str("Restoration needs a separate plan after this call settles and its dispatch deadline expires.\n");
    }
    if let Some(publication) = plan.publication() {
        if let Some(id) = &publication.after_removal {
            writeln!(output, "Restoration after removal: {}", escape(id)).expect("String write");
        }
        writeln!(output, "Root: {}\nRoot module: {}\nStore: {}\nStore module: {}\nRelease: {} / {}\nPublished receipt retained: {}",
            escape(&publication.root.canister), escape(&publication.root.module),
            escape(&publication.store.canister), escape(&publication.store.module),
            escape(&publication.release.name), escape(&publication.release.version), matches!(&state.detail, StatusDetail::Publication { publication: Some(_) })).expect("String write");
    }
    for custody in plan.maintenance().into_iter().flatten() {
        writeln!(
            output,
            "Preserve: child {} creation and {} completed child upgrade(s)",
            escape(custody.creation.worker.as_deref().unwrap_or("unknown")),
            custody.upgrades.len()
        )
        .expect("writing to String");
    }
    if let Some(store) = plan.maintenance_store() {
        writeln!(
            output,
            "Preserved Store: {}\nStore module: {}\nRetained artifact pins: {}",
            escape(&store.store.canister),
            escape(&store.store.module),
            store
                .sources
                .iter()
                .flatten()
                .map(|s| 1 + s.upgrades.len())
                .sum::<usize>()
        )
        .expect("String write");
    }
    if let Some(owner) = plan.management() {
        writeln!(
            output,
            "Root: {}\nRoot module: {}\nOperator: {}\nAutonomous dispatch: {}",
            escape(&owner.root),
            escape(&owner.root_module),
            escape(&plan.controller),
            dispatch(plan.created_at_ns, owner.execute_at_ns)
        )
        .expect("writing to String");
        if let Some(source) = &owner.source {
            writeln!(
                output,
                "Artifact source: Store {}\nStore module: {}\nRelease: {} / {}",
                escape(&source.store.canister),
                escape(&source.store.module),
                escape(&source.manifest.release.name),
                escape(&source.manifest.release.version)
            )
            .expect("String write");
        }
        output.push_str(&management::render(
            state.management.as_ref(),
            &state.detail,
            receipt.is_some(),
        ));
    }
    if let Some(target) = plan.target() {
        writeln!(output, "Upgrade target: {}", escape(target)).expect("writing to String");
    }
    if let Some(module) = plan.previous_module() {
        writeln!(output, "Required previous module: {}", escape(module))
            .expect("writing to String");
    }
    let store_upgrade = plan.store_upgrade_root().is_some();
    let rows: Vec<_> = if store_upgrade {
        // Scans retain many read/continuation pairs. Summarize their states here;
        // the original per-attempt evidence remains in Host JSON and the journal.
        let mut counts = BTreeMap::<&str, usize>::new();
        for evidence in effects.values().filter(|s| s.as_str() != "not_submitted") {
            *counts.entry(evidence.as_str()).or_default() += 1;
        }
        if counts.is_empty() {
            vec![["No retained request attempts".into(), "0".into()]]
        } else {
            counts
                .into_iter()
                .map(|(state, count)| {
                    [
                        evidence_description("", state, false).into(),
                        count.to_string(),
                    ]
                })
                .collect()
        }
    } else {
        effects
            .iter()
            .map(|(effect, evidence)| {
                [
                    effect.clone(),
                    evidence_description(effect, evidence, plan.target().is_some()).into(),
                ]
            })
            .collect()
    };
    writeln!(
        output,
        "\n{}",
        plain_table(
            if store_upgrade {
                &["RETAINED EVIDENCE", "ATTEMPTS"]
            } else {
                &["EFFECT", "RETAINED EVIDENCE"]
            },
            &rows,
            &[
                Alignment::Left,
                if store_upgrade {
                    Alignment::Right
                } else {
                    Alignment::Left
                }
            ],
        )
    )
    .expect("writing to String");
    if let Some(receipt) = receipt {
        writeln!(
            output,
            "Receipt: retained (historical deployment evidence)\n  Operation: {}\n  Canister: {}\n  Module: {}\n  Controller: {}",
            escape(&receipt.operation),
            escape(&receipt.canister),
            escape(&receipt.module_hash),
            escape(&receipt.controller),
        )
        .expect("writing to String");
    } else if plan.auth().is_none() && plan.publication().is_none() && plan.removal().is_none() {
        output.push_str("Receipt: none retained; deployment completion is not established.\n");
    }
    if effects
        .values()
        .any(|value| value == "signed_attempt_retained_outcome_unknown")
    {
        output.push_str("An uncertain attempt may have executed. Preserve this operation and its signed attempt; expiry does not establish failure or authorize a replacement.\n");
    }
    output.push_str("Current code, controllers, readiness and balances are not checked here.\n");
    Ok(output)
}

fn render_fleet(state: &canic_host::FleetStatus, journal: &Path) -> Result<String> {
    let mut output = fleet_preview(&state.operation, &state.fleet_plan, journal)?;
    if state.cancellation.is_some() {
        output.push_str("Cancelled before effect preparation. This approval cannot apply.\n");
        return Ok(output);
    }
    let evidence = |step: Option<&CanisterStatus>| -> String {
        let Some(step) = step else {
            return "Not yet admitted".into();
        };
        if step.receipt.is_some() {
            return "Completion receipt retained".into();
        }
        match &step.management {
            Some(ManagementStatus::Creation(s)) => format!("{:?}; no completion receipt", s.phase),
            Some(ManagementStatus::Upgrade(s)) => format!("{:?}; no completion receipt", s.phase),
            None => "No completion receipt; inspect host effects with --json".into(),
        }
    };
    let mut rows = vec![["Root".into(), evidence(Some(&state.root))]];
    if let Some(store) = &state.store {
        rows.push([store.name.clone(), evidence(store.state.as_ref())]);
    }
    for child in &state.children {
        if let Some(publication) = &child.publication {
            rows.push([
                format!("{} release", child.name),
                if matches!(
                    &publication.detail,
                    StatusDetail::Publication {
                        publication: Some(_)
                    }
                ) {
                    "Published release evidence retained"
                } else {
                    "Publication unfinished; resume fleet apply"
                }
                .into(),
            ]);
        }
        rows.push([child.name.clone(), evidence(child.state.as_ref())]);
    }
    output.push_str(&plain_table(
        &["STEP", "RETAINED EVIDENCE"],
        &rows,
        &[Alignment::Left; 2],
    ));
    output
        .push_str("\n\nLocal evidence only. Resume this same fleet operation with fleet apply.\n");
    for child in &state.children {
        if let Some(step) = &child.state
            && step.management.is_some()
        {
            writeln!(output, "\nChild: {}", escape(&child.name)).expect("String write");
            output.push_str(&management::render(
                step.management.as_ref(),
                &step.detail,
                step.receipt.is_some(),
            ));
        }
    }
    Ok(output)
}

fn evidence_description<'a>(effect: &str, evidence: &'a str, upgrade: bool) -> &'a str {
    match evidence {
        "not_submitted" if effect == "create" && upgrade => {
            "Not applicable to this upgrade (not submitted)"
        }
        "not_submitted" => "Not submitted",
        "authority_retained_not_signed" => "Recovery request prepared; not signed",
        "signed_attempt_retained_outcome_unknown" => "Signed attempt retained; outcome UNKNOWN",
        "reply_retained" => "Reply retained; not itself proof of success",
        "rejection_retained" => "Rejection retained",
        _ => evidence,
    }
}

pub(crate) fn fleet_preview(
    id: &str,
    plan: &canic_host::FleetPlan,
    journal: &Path,
) -> Result<String> {
    if plan.digest()? != id {
        return Err(Error::Invalid("fleet approval digest mismatch".into()));
    }
    let root = plan
        .root
        .deployment
        .as_ref()
        .ok_or_else(|| Error::Invalid("missing Root slot".into()))?;
    let mut rows = vec![[
        crate::topology::ROOT_BADGE.into(),
        "operator".into(),
        format!("{} debit ceiling", cycles(plan.root.creation_cycles())),
    ]];
    let mut modules = format!("Root module: {}\n", plan.root.artifact.sha256);
    if let Some(store) = &plan.store {
        rows.push([
            format!("├─ {}", crate::topology::STORE_BADGE),
            format!("{}.result.canister", root.canister),
            format!("{} separate debit ceiling", cycles(store.creation_cycles)),
        ]);
        writeln!(
            modules,
            "{} module: {}",
            escape(&store.deployment.canister),
            store.artifact.sha256
        )
        .expect("String write");
    }
    writeln!(
        modules,
        "Total operator debit ceiling: {}",
        cycles(plan.creation_cycles())
    )
    .expect("String write");
    if let canic_host::PlanAction::Create {
        bootstrap: Some(policy),
        ..
    } = plan.root.action
    {
        writeln!(
            modules,
            "Subnet child limit: {}{}",
            policy.max_children,
            if policy.allow_large_subnet {
                " (large subnet approved)"
            } else {
                ""
            }
        )
        .expect("String write");
    }
    for (index, child) in plan.children.iter().enumerate() {
        let branch = if index + 1 < plan.children.len() {
            "├─"
        } else {
            "└─"
        };
        rows.push([
            format!("{branch} {}", child.deployment.canister),
            format!("{}.result.canister", root.canister),
            format!("{} from Root", cycles(child.creation_cycles)),
        ]);
        writeln!(
            modules,
            "{} module: {}",
            escape(&child.deployment.canister),
            child.artifact.sha256
        )
        .expect("String write");
        if child.receiver_role.is_some() {
            writeln!(
                modules,
                "  Role verifier: pin the signing key from this Root at install"
            )
            .expect("String write");
        }
        if let Some(release) = &child.release {
            writeln!(
                modules,
                "  Store release: {} / {} (publication before child admission)",
                escape(&release.release.name),
                escape(&release.release.version)
            )
            .expect("String write");
        }
    }
    Ok(format!(
        "CANIC FLEET PLAN\nOperation: {}\nApp: {}\nFleet: {}\nNetwork: {}\nOperator: {}\nJournal: {}\n\n{}\n\n{}Child dispatch: {}\nChild effect deadline: {}\n\n",
        escape(id),
        escape(&root.app),
        escape(&root.fleet),
        escape(&match &plan.root.network {
            Network::Mainnet => "IC mainnet".into(),
            Network::Local { url, .. } => format!("Local replica: {url}"),
        }),
        escape(&plan.root.controller),
        escape(&journal.to_string_lossy()),
        plain_table(
            &["CANISTER", "CONTROLLER", "BUDGET"],
            &rows,
            &[Alignment::Left; 3]
        ),
        modules,
        dispatch(plan.root.created_at_ns, plan.dispatch_at_ns()),
        deadline(plan.root.expires_at_ns)
    ))
}

fn dispatch(created: u64, earliest: u64) -> String {
    if earliest == created {
        "when approved prerequisites are ready".into()
    } else {
        format!("{} (original admission cutoff)", utc(earliest))
    }
}

fn deadline(ns: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_secs());
    let seconds = ns / 1_000_000_000;
    let relative = if seconds > now {
        format!("in {}m {}s", (seconds - now) / 60, (seconds - now) % 60)
    } else {
        "expired".into()
    };
    format!("{} ({relative})", utc(ns))
}

fn utc(ns: u64) -> String {
    // Every u64 nanosecond timestamp fits within Time's supported calendar range.
    let date = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ns))
        .expect("u64 nanoseconds are within the supported UTC range");
    format!(
        "{} {:02}:{:02}:{:02} UTC",
        date.date(),
        date.hour(),
        date.minute(),
        date.second()
    )
}

fn cycles(value: u64) -> String {
    let (divisor, width, unit) = if value >= 1_000_000_000_000 {
        (1_000_000_000_000, 12, "T")
    } else if value >= 1_000_000_000 {
        (1_000_000_000, 9, "B")
    } else {
        return format!("{value} cycles");
    };
    // Budgets are exact amounts: never round a fractional unit up to more authority.
    let fraction = format!("{:0width$}", value % divisor);
    let fraction = fraction.trim_end_matches('0');
    let whole = value / divisor;
    let amount = if fraction.is_empty() {
        whole.to_string()
    } else {
        format!("{whole}.{fraction}")
    };
    format!("{amount}{unit} cycles ({value})")
}
