//! Select a bound principal and inspect its exact local module.
use crate::Output;
use canic_host::{Fleet, Result};
use ic_host_text::{Alignment, escape, plain_table};
use std::{fmt::Write as _, path::Path};

pub(crate) async fn inspect(
    snapshot: &Fleet,
    config_path: &Path,
    id: &str,
    identity: Option<&Path>,
    wasm: Option<&Path>,
    json: bool,
) -> Result<Output> {
    let config = snapshot.configuration.as_ref().ok_or_else(|| {
        canic_host::Error::Invalid("canister inspection requires a fleet configuration".into())
    })?;
    let binding = config.config.network.binding_id()?;
    let target = snapshot
        .canisters
        .iter()
        .find(|target| target.network_id == binding && target.canister == id)
        .ok_or_else(|| {
            canic_host::Error::Invalid(format!(
                "principal {id} was not found in this fleet inventory: {:?}",
                snapshot.issues
            ))
        })?;
    let declared = config
        .config
        .canisters
        .values()
        .find(|slot| slot.id.as_deref() == Some(id))
        .or_else(|| {
            let role = target.installation.as_ref()?.role.as_deref()?;
            let mut slots = config
                .config
                .canisters
                .values()
                .filter(|slot| slot.role.as_deref() == Some(role));
            let slot = slots.next()?;
            slots.next().is_none().then_some(slot)
        });
    let path = wasm.map(Path::to_owned).or_else(|| {
        declared?.wasm.as_ref().map(|path| {
            config_path
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join(path)
        })
    });
    let client = canic_host::InspectionClient::new(&config.config.network, id, identity)?;
    let mut view = serde_json::json!({"canister":target,"caller":client.caller().to_text(),"diagnostics":client.diagnostics().await,"artifact":path,"inventory_issues":snapshot.issues});
    match (path, &target.module_hash) {
        (Some(path), Some(hash)) => match canic_host::installed_endpoints(&path, hash) {
            Ok(methods) => view["endpoints"] = serde_json::json!(methods),
            Err(error) => view["artifact_error"] = error.to_string().into(),
        },
        _ => view["artifact_error"] = "local artifact or certified module hash unavailable".into(),
    }

    if json {
        return Ok(Output::Json(view));
    }
    let mut text = format!(
        "{}  {}\n",
        escape(id),
        escape(target.module_hash.as_deref().unwrap_or("module unknown"))
    );
    if let Some(error) = view["artifact_error"].as_str() {
        writeln!(text, "Artifact: {}", escape(error)).unwrap();
    }
    if let Some(methods) = view["endpoints"].as_array() {
        let rows: Vec<_> = methods
            .iter()
            .map(|m| {
                let field = |key: &str| m[key].as_str().unwrap_or("—").to_owned();
                let bytes = |key: &str| {
                    m[key]["bytes"]
                        .as_u64()
                        .map_or_else(|| "—".into(), |n| n.to_string())
                };
                [
                    field("name"),
                    field("mode"),
                    field("argument"),
                    field("reply"),
                    bytes("request_limits"),
                    bytes("reply_limits"),
                ]
            })
            .collect();
        writeln!(
            text,
            "{}",
            plain_table(
                &[
                    "ENDPOINT",
                    "MODE",
                    "REQUEST",
                    "REPLY",
                    "REQ BYTES",
                    "REPLY BYTES"
                ],
                &rows,
                &[
                    Alignment::Left,
                    Alignment::Left,
                    Alignment::Left,
                    Alignment::Left,
                    Alignment::Right,
                    Alignment::Right
                ]
            )
        )
        .unwrap();
    }
    for name in ["readiness", "logs", "metrics"] {
        writeln!(text, "{name} {}", view["diagnostics"][name]).unwrap();
    }
    Ok(Output::Text(text))
}
