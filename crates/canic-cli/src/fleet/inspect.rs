//! Select a bound principal and inspect its exact local module.
use crate::Output;
use canic_host::{Fleet, Result};
use ic_host_text::{Alignment, escape, plain_table};
use std::{fmt::Write as _, path::Path};

pub(crate) async fn inspect(
    snapshot: &Fleet,
    config_path: &Path,
    id: &str,
    session: &canic_host::InspectionSession,
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
    #[derive(serde::Serialize)]
    struct View<'a> {
        canister: &'a canic_host::FleetCanister,
        caller: String,
        diagnostics: canic_host::Diagnostics,
        artifact: Option<std::path::PathBuf>,
        inventory_issues: &'a [String],
        #[serde(skip_serializing_if = "Option::is_none")]
        endpoints: Option<Vec<canic_host::WasmEndpoint>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        artifact_error: Option<String>,
    }
    let client = session.client(&config.config.network, id)?;
    let result = match (&path, &target.module_hash) {
        (Some(path), Some(hash)) => {
            canic_host::installed_endpoints(path, hash).map_err(|e| e.to_string())
        }
        _ => Err("local artifact or certified module hash unavailable".into()),
    };
    let (endpoints, artifact_error) = match result {
        Ok(methods) => (Some(methods), None),
        Err(error) => (None, Some(error)),
    };
    let view = View {
        canister: target,
        caller: client.caller().to_text(),
        diagnostics: client.diagnostics().await,
        artifact: path,
        inventory_issues: &snapshot.issues,
        endpoints,
        artifact_error,
    };
    if json {
        return Ok(Output::Json(serde_json::json!(view)));
    }
    let mut text = format!(
        "{}  {}\n",
        escape(id),
        escape(target.module_hash.as_deref().unwrap_or("module unknown"))
    );
    if let Some(error) = view.artifact_error.as_deref() {
        writeln!(text, "Artifact: {}", escape(error)).unwrap();
    }
    if let Some(methods) = view.endpoints.as_ref() {
        let rows: Vec<_> = methods
            .iter()
            .map(|m| {
                let bytes = |limits: &Option<canic_host::WasmLimits>| {
                    limits
                        .as_ref()
                        .map_or_else(|| "—".into(), |v| v.bytes.to_string())
                };
                [
                    m.name.clone(),
                    m.mode.into(),
                    m.argument.as_deref().unwrap_or("—").into(),
                    m.reply.as_deref().unwrap_or("—").into(),
                    bytes(&m.request_limits),
                    bytes(&m.reply_limits),
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
    writeln!(
        text,
        "readiness {}",
        serde_json::json!(view.diagnostics.readiness)
    )
    .unwrap();
    writeln!(text, "logs {}", serde_json::json!(view.diagnostics.logs)).unwrap();
    writeln!(
        text,
        "metrics {}",
        serde_json::json!(view.diagnostics.metrics)
    )
    .unwrap();
    Ok(Output::Text(text))
}
