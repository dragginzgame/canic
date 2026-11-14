//! Local declaration listings, independent of journals and live observations.
use crate::Output;
use canic_host::{Error, Network, Result};
use ic_host_text::table::{self, Alignment, Options};
use std::path::Path;

const TABLE_OPTIONS: Options = Options {
    column_gap: 2,
    header_separator: true,
};

pub(crate) fn canisters(path: &Path, app: &str, fleet: &str, json: bool) -> Result<Output> {
    let selected = canic_host::load_config(path)?.select(Some(app), Some(fleet))?;
    if json {
        return Ok(Output::Json(serde_json::json!({
            "scope":"local configuration only", "config_path":path, "configuration":selected,
        })));
    }
    let title = format!("Configured canister slots — app {app}, fleet {fleet}");
    if selected.config.canisters.is_empty() {
        return Ok(Output::Text(format!("{title}\nNo slots declared.\n")));
    }
    let rows: Vec<_> = selected
        .config
        .canisters
        .iter()
        .map(|(slot, config)| {
            [
                slot.clone(),
                config.id.clone().unwrap_or_else(|| "—".into()),
            ]
        })
        .collect();
    let table = table::render(
        &["SLOT", "CONFIGURED ID"],
        &rows,
        &[Alignment::Left, Alignment::Left],
        TABLE_OPTIONS,
    );
    Ok(Output::Text(format!(
        "{title}\n{table}\nConfiguration only; an unset ID does not mean undeployed. Use fleet show for deployment observations.\n"
    )))
}

pub(crate) fn list(path: &Path, app: Option<&str>, apps_only: bool, json: bool) -> Result<Output> {
    let config = canic_host::load_config(path)?;
    if let Some(app) = app
        && !config.apps.contains_key(app)
    {
        return Err(Error::Invalid(format!(
            "unknown app; choose from: {}",
            config.apps.keys().cloned().collect::<Vec<_>>().join(", ")
        )));
    }
    let mut rows = Vec::new();
    let mut app_rows = Vec::new();
    let mut fleet_rows = Vec::new();
    for (app_name, definition) in &config.apps {
        if app.is_some_and(|selected| selected != app_name) {
            continue;
        }
        if apps_only {
            rows.push(serde_json::json!({"app":app_name, "fleets":definition.fleets.len()}));
            app_rows.push([app_name.clone(), definition.fleets.len().to_string()]);
        } else {
            for (fleet, definition) in &definition.fleets {
                let network = match &definition.network {
                    Network::Mainnet => "ic / mainnet".into(),
                    Network::Local { url, .. } => format!("local / {url}"),
                };
                rows.push(serde_json::json!({"app":app_name, "fleet":fleet, "network":definition.network, "declared_canisters":definition.canisters.len()}));
                fleet_rows.push([
                    app_name.clone(),
                    fleet.clone(),
                    definition.canisters.len().to_string(),
                    network,
                ]);
            }
        }
    }
    if json {
        let mut value = serde_json::json!({"scope":"local configuration only", "config_path":path});
        value[if apps_only { "apps" } else { "fleets" }] = serde_json::json!(rows);
        Ok(Output::Json(value))
    } else {
        let (title, rendered) = if apps_only {
            (
                "Configured apps (local declarations)",
                table::render(
                    &["APP", "FLEETS"],
                    &app_rows,
                    &[Alignment::Left, Alignment::Right],
                    TABLE_OPTIONS,
                ),
            )
        } else {
            (
                "Configured fleets (local declarations; slots are not a live canister count)",
                table::render(
                    &["APP", "FLEET", "SLOTS", "NETWORK"],
                    &fleet_rows,
                    &[
                        Alignment::Left,
                        Alignment::Left,
                        Alignment::Right,
                        Alignment::Left,
                    ],
                    TABLE_OPTIONS,
                ),
            )
        };
        Ok(Output::Text(format!("{title}\n{rendered}\n")))
    }
}
