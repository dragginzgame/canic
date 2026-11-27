//! Local declaration listings, independent of journals and live observations.
use crate::Output;
use canic_host::{FleetNetwork, Network, Result};
use ic_host_text::{Alignment, plain_table};
use std::path::Path;

pub(crate) fn app_canisters(path: &Path, app: Option<&str>, json: bool) -> Result<Output> {
    let config = canic_host::load_config(path)?;
    let (app, definition) = config.select_app(app)?;
    if json {
        return Ok(Output::Json(
            serde_json::json!({"scope":"local app definitions only", "config_path":path, "app":app, "canisters":definition.canisters}),
        ));
    }
    let rows: Vec<_> = crate::topology::rows(
        definition
            .canisters
            .iter()
            .map(|(name, slot)| (name.as_str(), slot.parent.as_deref())),
    )
    .into_iter()
    .map(|(name, prefix)| {
        let item = &definition.canisters[name];
        [
            format!("{prefix}{name}"),
            crate::topology::role_label(item.role.as_deref().unwrap_or("—")).into(),
            item.wasm
                .as_ref()
                .map_or_else(|| "—".into(), |path| path.display().to_string()),
        ]
    })
    .collect();
    let table = plain_table(
        &["CANISTER", "ROLE", "WASM INPUT"],
        &rows,
        &[Alignment::Left; 3],
    );
    Ok(Output::Text(format!("App: {app}\n\n{table}\n\n")))
}

pub(crate) fn canisters(
    path: &Path,
    app: Option<&str>,
    fleet: Option<&str>,
    json: bool,
) -> Result<Output> {
    let config = canic_host::load_config(path)?;
    let (app, definition) = config.select_app(app)?;
    let (fleet, declaration) = definition.fleet_declaration(fleet)?;
    let canisters = definition.resolve_slots(declaration)?;
    if json {
        let configuration = match declaration.network {
            FleetNetwork::Bound(_) => serde_json::json!(config.select(Some(app), Some(fleet))?),
            FleetNetwork::Unbound(_) => {
                let mut declared = serde_json::json!(declaration);
                declared["canisters"] = serde_json::json!(canisters);
                serde_json::json!({"app":app, "fleet":fleet, "config":declared})
            }
        };
        return Ok(Output::Json(serde_json::json!({
            "scope":"local configuration only", "config_path":path, "configuration":configuration,
        })));
    }
    let title = format!("App: {app} · Fleet: {fleet}");
    if canisters.is_empty() {
        return Ok(Output::Text(format!(
            "{title}\n\nNo canisters defined.\n\n"
        )));
    }
    let rows: Vec<_> = crate::topology::rows(
        canisters
            .iter()
            .map(|(name, slot)| (name.as_str(), slot.parent.as_deref())),
    )
    .into_iter()
    .map(|(slot, prefix)| {
        let config = &canisters[slot];
        [
            format!("{prefix}{slot}"),
            crate::topology::role_label(config.role.as_deref().unwrap_or("—")).into(),
            config.id.clone().unwrap_or_else(|| "—".into()),
        ]
    })
    .collect();
    let table = plain_table(
        &["CANISTER", "ROLE", "PRINCIPAL"],
        &rows,
        &[Alignment::Left; 3],
    );
    let mut output = format!("{title}\n\n{table}\n\n");
    if !declaration.policy.top_up_children.is_empty() {
        let edges: Vec<_> = declaration
            .policy
            .top_up_children
            .iter()
            .map(|rule| {
                let child = rule.child();
                let parent = canisters[child]
                    .parent
                    .as_deref()
                    .expect("validated direct child");
                match rule {
                    canic_host::ChildTopUp::Eligible(_) => format!("{parent} → {child}"),
                    canic_host::ChildTopUp::Rule {
                        below_cycles,
                        amount_cycles,
                        ..
                    } => format!(
                        "{parent} → {child} (below {below_cycles} cycles, add {amount_cycles})"
                    ),
                }
            })
            .collect();
        output.push_str(&format!(
            "Eligible for parent top-ups: {}\n",
            edges.join(", ")
        ));
    }
    Ok(Output::Text(output))
}

pub(crate) fn list(
    path: Option<&Path>,
    app: Option<&str>,
    apps_only: bool,
    json: bool,
) -> Result<Output> {
    let configs = canic_host::discover_configs(Path::new(""), path)?;
    if let Some(app) = app
        && !configs
            .iter()
            .any(|item| item.config.apps.contains_key(app))
    {
        let names = configs
            .iter()
            .flat_map(|item| item.config.apps.keys().cloned())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(canic_host::Error::Invalid(format!(
            "unknown app {app}; select --app from: {names}"
        )));
    }
    let mut rows = Vec::new();
    let mut app_rows = Vec::new();
    let mut fleet_rows = Vec::new();
    for item in &configs {
        let config = &item.config;
        for (app_name, definition) in &config.apps {
            if app.is_some_and(|selected| selected != app_name) {
                continue;
            }
            if apps_only {
                rows.push(serde_json::json!({"app":app_name, "fleets":definition.fleets.len(), "config_path":item.path}));
                app_rows.push([
                    app_name.clone(),
                    definition.fleets.len().to_string(),
                    item.path.display().to_string(),
                ]);
            } else {
                for (fleet, definition) in &definition.fleets {
                    let network = match &definition.network {
                        FleetNetwork::Bound(Network::Mainnet) => "ic / mainnet".into(),
                        FleetNetwork::Unbound(_) => "local / unbound".into(),
                        FleetNetwork::Bound(Network::Local { url, .. }) => format!("local / {url}"),
                    };
                    let count = config.apps[app_name].canisters.len();
                    rows.push(serde_json::json!({"app":app_name, "fleet":fleet, "network":definition.network, "declared_canisters":count, "config_path":item.path}));
                    fleet_rows.push([
                        app_name.clone(),
                        fleet.clone(),
                        count.to_string(),
                        network,
                        item.path.display().to_string(),
                    ]);
                }
            }
        }
    }
    if json {
        let paths: Vec<_> = configs.iter().map(|item| &item.path).collect();
        let path = if paths.len() == 1 {
            Some(paths[0])
        } else {
            None
        };
        let mut value = serde_json::json!({"scope":"local configuration only", "config_path":path, "config_paths":paths});
        value[if apps_only { "apps" } else { "fleets" }] = serde_json::json!(rows);
        Ok(Output::Json(value))
    } else {
        let (title, rendered) = if apps_only {
            (
                "Configured apps (local declarations)",
                plain_table(
                    &["APP", "FLEETS", "CONFIG"],
                    &app_rows,
                    &[Alignment::Left, Alignment::Right, Alignment::Left],
                ),
            )
        } else {
            (
                "Configured fleets (local declarations; slots are not a live canister count)",
                plain_table(
                    &["APP", "FLEET", "SLOTS", "NETWORK", "CONFIG"],
                    &fleet_rows,
                    &[
                        Alignment::Left,
                        Alignment::Left,
                        Alignment::Right,
                        Alignment::Left,
                        Alignment::Left,
                    ],
                ),
            )
        };
        Ok(Output::Text(format!("{title}\n{rendered}\n")))
    }
}
