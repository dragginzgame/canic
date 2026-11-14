//! Local declaration listings, independent of journals and live observations.
use crate::Output;
use canic_host::{Error, Network, Result};
use ic_host_text::table::{self, Alignment, Options};
use std::path::Path;

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
        let options = Options {
            column_gap: 2,
            header_separator: true,
        };
        let (title, rendered) = if apps_only {
            (
                "Configured apps (local declarations)",
                table::render(
                    &["APP", "FLEETS"],
                    &app_rows,
                    &[Alignment::Left, Alignment::Right],
                    options,
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
                    options,
                ),
            )
        };
        Ok(Output::Text(format!("{title}\n{rendered}\n")))
    }
}
