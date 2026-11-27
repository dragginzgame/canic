//! App command context requires a declared fleet; there is no legacy journal-only mode.
use crate::args::CallArgs;
use canic_host::{Error, Result, application::codec};
use std::path::Path;

pub(crate) async fn run(args: &CallArgs) -> Result<serde_json::Value> {
    let CallArgs {
        selection,
        state_dir,
        canister,
        identity,
        signer,
        command,
    } = args;
    let (config_path, config, selected) = selection.resolve()?;
    let journal = state_dir
        .clone()
        .unwrap_or_else(|| selected.journal_path(&config_path));
    let (_, app) = config.select_app(Some(&selected.app))?;
    let definition = app
        .canisters
        .get(canister)
        .ok_or_else(|| Error::Invalid("unknown application canister".into()))?;
    if definition
        .build
        .as_ref()
        .and_then(|b| b.client.as_ref())
        .is_none()
    {
        return Err(Error::Invalid(
            "app canister has no build.client command codec".into(),
        ));
    }
    let client = codec::output(
        &config_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(definition.wasm.as_ref().expect("validated build")),
    )
    .canonicalize()
    .map_err(|e| Error::Supplier(Box::new(e)))?;
    let request = codec::encode(&client, command)?;
    let caller = canic_host::signing_principal(identity, &signer.icp_bin)?;
    let (digest, plan) = canic_host::application::prepare(
        &journal,
        &selected,
        canister,
        caller,
        command.clone(),
        request,
    )?;
    let approval = plan.application_call().expect("prepared application call");
    eprintln!(
        "Call {} ({}) on {} as {}\nOperation: {}\nRequest: {} ({} bytes)",
        ic_host_text::escape(&approval.command[0]),
        ic_host_text::escape(&approval.method),
        approval.canister,
        plan.controller,
        digest,
        plan.artifact.sha256,
        plan.artifact.bytes
    );
    let canic_host::ApplyOutcome::Call { call } =
        canic_host::apply(&journal, &digest, identity, &signer.icp_bin).await?
    else {
        unreachable!("application call outcome")
    };
    let result = codec::decode(&client, &call.bytes)?;
    Ok(
        serde_json::json!({"operation": digest, "canister": approval.canister, "caller": plan.controller, "result": result}),
    )
}
