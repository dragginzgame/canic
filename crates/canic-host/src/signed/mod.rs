//! ICP CLI owns keys and signing; Canic owns exact-attempt custody and submission.

#[cfg(test)]
pub(crate) mod tests;

use crate::{
    Error, Plan, Result,
    network::{bounded, principal},
    now, store, supplier,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use ic_platform::{
    ingress::{self, Content},
    native::{Dispatch, PreparedRequestStatus, PreparedUpdate, ReadClient, RequestStatus},
};
use ic_platform_types::Principal;
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};

#[derive(Deserialize)]
struct Message {
    format: String,
    version: u32,
    request: Request,
    network: SignedNetwork,
    destination: Destination,
}
#[derive(Deserialize)]
struct Request {
    #[serde(rename = "type")]
    kind: String,
    envelope: String,
    request_id: String,
    status_check: String,
}
#[derive(Deserialize)]
struct SignedNetwork {
    url: String,
    root_key: String,
}
#[derive(Deserialize)]
struct Destination {
    canister: String,
}

pub(crate) struct Call<'a> {
    pub canister: Principal,
    pub effective: Principal,
    pub method: &'a str,
    pub args: Vec<u8>,
}

struct Attempt {
    update: PreparedUpdate,
    status: PreparedRequestStatus,
}

struct Authority<'a> {
    network: &'a crate::Network,
    controller: &'a str,
    expires_at_ns: u64,
}

impl<'a> From<&'a Plan> for Authority<'a> {
    fn from(plan: &'a Plan) -> Self {
        Self {
            network: &plan.network,
            controller: &plan.controller,
            expires_at_ns: plan.expires_at_ns,
        }
    }
}

fn admit(bytes: &[u8], plan: &Authority<'_>, call: &Call<'_>) -> Result<Attempt> {
    let message: Message = serde_json::from_slice(bytes).map_err(supplier)?;
    if message.format != "icp-signed-message"
        || message.version != 1
        || message.request.kind != "update"
        || message.network.url != plan.network.url()
        || message.network.root_key != plan.network.root()
        || (principal(&message.destination.canister)? != call.effective
            && !(call.canister == Principal::management_canister()
                && principal(&message.destination.canister)? == Principal::management_canister()))
    {
        return Err(Error::Invalid(
            "signed-message transport differs from approved plan".into(),
        ));
    }
    let envelope = BASE64_STANDARD
        .decode(&message.request.envelope)
        .map_err(supplier)?;
    // Raw-argument signing can leave the unsigned routing hint at aaaaa-aa.
    // Route management calls only to the effective principal derived from the
    // approved plan/result; the signed method and exact argument bytes bind it.
    let content = ingress::decode_envelope(&envelope, store::MAX_RECORD)
        .map_err(supplier)?
        .value
        .content;
    // Select the profile from approved authority, never from a failed signature.
    // Platform owns authentication and exact status-path binding for both profiles.
    let anonymous = principal(plan.controller)? == Principal::anonymous();
    let effective = call.effective;
    let prepared = if anonymous {
        PreparedUpdate::from_anonymous_bytes(&envelope, effective, store::MAX_RECORD)
    } else {
        PreparedUpdate::from_bytes(&envelope, effective, store::MAX_RECORD)
    }
    .map_err(supplier)?;
    let expiry = match content {
        Content::Call {
            sender,
            canister_id,
            method_name,
            arg,
            ingress_expiry,
            ..
        } if sender.as_slice() == principal(plan.controller)?.as_slice()
            && canister_id.as_slice() == call.canister.as_slice()
            && method_name == call.method
            && arg == call.args
            && ingress_expiry <= plan.expires_at_ns =>
        {
            ingress_expiry
        }
        _ => {
            return Err(Error::Invalid(
                "signed call differs from approved sender, effect or arguments".into(),
            ));
        }
    };
    if crate::hex(prepared.request_id()) != message.request.request_id {
        return Err(Error::Invalid("signed request ID mismatch".into()));
    }
    let status_check = BASE64_STANDARD
        .decode(&message.request.status_check)
        .map_err(supplier)?;
    let status = if anonymous {
        PreparedRequestStatus::from_anonymous_bytes(&status_check, &prepared, store::MAX_RECORD)
    } else {
        PreparedRequestStatus::from_bytes(&status_check, &prepared, store::MAX_RECORD)
    }
    .map_err(supplier)?;
    // Read authority cannot extend the original operation's signing window.
    if status.expiry() != expiry {
        return Err(Error::Invalid(
            "signed status check does not bind the same attempt".into(),
        ));
    }
    Ok(Attempt {
        update: prepared,
        status,
    })
}

fn sign(
    path: &Path,
    plan: &Authority<'_>,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
) -> Result<Vec<u8>> {
    if now()? + 300_000_000_000 > plan.expires_at_ns {
        return Err(Error::Blocked(
            "plan has less than five minutes left; no new effect was signed".into(),
        ));
    }
    admit_signer(icp)?;
    store::put(&path.join("arguments.bin"), &call.args)?;
    let output = path.join("signing.json");
    let mut command = Command::new(icp);
    command
        .args([
            "canister",
            "call",
            &call.canister.to_text(),
            call.method,
            "--args-file",
        ])
        .arg(path.join("arguments.bin"))
        .args([
            "--args-format",
            "bin",
            "--identity",
            identity,
            "--network",
            plan.network.url(),
            "--root-key",
            plan.network.root(),
            "--sign-only",
        ])
        .arg(&output)
        .stdin(Stdio::inherit())
        .stdout(Stdio::from(std::io::stderr()))
        .stderr(Stdio::inherit());
    // Interactive password input is inherited, never captured. No network or
    // paid effect is performed by sign-only; a failed signer is not submitted.
    let result = command.status().map_err(supplier)?;
    if !result.success() {
        return Err(Error::Invalid(format!("ICP signing failed with {result}")));
    }
    let bytes = store::read(&output, store::MAX_RECORD)?;
    admit(&bytes, plan, call)?;
    Ok(bytes)
}

#[derive(Serialize)]
struct Rejection {
    code: String,
    message: String,
}

pub(crate) async fn execute(
    path: &Path,
    plan: &Plan,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    // Application mutations are submitted at most once. Resumption can only
    // observe their original signed request; management keeps its existing policy.
    let dispatch = plan.application_call().is_none()
        || !path.join("signed.json").try_exists().map_err(supplier)?;
    execute_with(
        path,
        &Authority::from(plan),
        call,
        identity,
        icp,
        client,
        dispatch,
    )
    .await
}

/// Read only the typed metadata selected by the approved operation.
/// Its retained deadline cannot be used to renew any mutation's signed attempt.
pub(crate) async fn observe_operation(
    path: &Path,
    plan: &Plan,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    observe_metadata(path, plan, None, identity, icp, client).await
}

pub(crate) async fn observe_store(
    path: &Path,
    plan: &Plan,
    canister: Principal,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    observe_metadata(path, plan, Some(canister), identity, icp, client).await
}

// Only these typed read contracts may obtain fresh authority, never mutation calls.
fn metadata_call(plan: &Plan, store: Option<Principal>, deadline: u64) -> Result<Call<'static>> {
    match store {
        Some(canister) => crate::store_bootstrap::observation(plan, canister, deadline),
        None if plan.store_upgrade_root().is_some() => crate::store_upgrade::observation(plan),
        None => crate::publication::observation(plan, deadline),
    }
}

// A retry must use the deadline recorded before the original submission.
fn retained_deadline(path: &Path) -> Result<u64> {
    let directory = crate::output_publication::prepare(path, 0o700)?;
    let authority = path.join("authority.json");
    if authority.try_exists().map_err(supplier)? {
        return store::json(&authority);
    }
    let deadline = now()? + 600_000_000_000;
    store::put_json_in(
        &directory,
        std::ffi::OsStr::new("authority.json"),
        &deadline,
    )?;
    Ok(deadline)
}

async fn observe_metadata(
    path: &Path,
    plan: &Plan,
    store: Option<Principal>,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    let expires_at_ns = retained_deadline(path)?;
    let dispatch_deadline = expires_at_ns
        .checked_sub(300_000_000_000)
        .ok_or_else(|| Error::Invalid("invalid metadata observation authority".into()))?;
    let call = metadata_call(plan, store, dispatch_deadline)?;
    execute_with(
        path,
        &Authority {
            network: &plan.network,
            controller: &plan.controller,
            expires_at_ns,
        },
        &call,
        identity,
        icp,
        client,
        true,
    )
    .await
}

/// Admit retained operation evidence against its exact signed read contract.
pub(crate) fn retained_operation(path: &Path, plan: &Plan) -> Result<Vec<u8>> {
    retained_metadata(path, plan, None)
}

pub(crate) fn retained_store(path: &Path, plan: &Plan, canister: Principal) -> Result<Vec<u8>> {
    retained_metadata(path, plan, Some(canister))
}

fn retained_metadata(path: &Path, plan: &Plan, store: Option<Principal>) -> Result<Vec<u8>> {
    let expires_at_ns: u64 = store::json(&path.join("authority.json"))?;
    let deadline = expires_at_ns
        .checked_sub(300_000_000_000)
        .ok_or_else(|| Error::Invalid("invalid metadata observation authority".into()))?;
    let call = metadata_call(plan, store, deadline)?;
    admit(
        &store::read(&path.join("signed.json"), store::MAX_RECORD)?,
        &Authority {
            network: &plan.network,
            controller: &plan.controller,
            expires_at_ns,
        },
        &call,
    )?;
    store::read(&path.join("reply.bin"), 2 * 1024 * 1024)
}

/// Check original effect custody without resubmission, including after authority handover.
pub(crate) fn validate_retained(path: &Path, plan: &Plan, call: &Call<'_>) -> Result<()> {
    admit(
        &store::read(&path.join("signed.json"), store::MAX_RECORD)?,
        &Authority::from(plan),
        call,
    )?;
    Ok(())
}

/// Only the selected Root observation contract receives fresh read authority after plan expiry.
pub(crate) async fn observe(
    path: &Path,
    plan: &Plan,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    let (root, method, args) = if plan.maintenance().is_some() {
        let args = crate::maintenance::observation_args(plan, call.method)?;
        (
            principal(
                plan.target()
                    .map(String::as_str)
                    .expect("validated maintenance target"),
            )?,
            call.method,
            args,
        )
    } else {
        let selected = crate::managed::observation(plan)?;
        (selected.canister, selected.method, selected.args)
    };
    if call.canister != root
        || call.effective != call.canister
        || call.method != method
        || call.args != args
    {
        return Err(Error::Invalid(
            "read authority cannot authorize another method or payload".into(),
        ));
    }
    execute_with(
        path,
        &Authority {
            network: &plan.network,
            controller: &plan.controller,
            expires_at_ns: now()? + 600_000_000_000,
        },
        call,
        identity,
        icp,
        client,
        true,
    )
    .await
}

/// Explicit operator request to reconcile only the plan's exact retained approval.
/// Its short submission window is independent of, and cannot extend, effect authority.
pub(crate) async fn reconcile(
    path: &Path,
    plan: &Plan,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
) -> Result<Vec<u8>> {
    let call = if plan.store_upgrade_root().is_some() {
        crate::store_upgrade::recovery(plan)?
    } else {
        crate::managed::recovery::call(plan)?
    };
    let expires_at_ns = retained_deadline(path)?;
    execute_with(
        path,
        &Authority {
            network: &plan.network,
            controller: &plan.controller,
            expires_at_ns,
        },
        &call,
        identity,
        icp,
        client,
        true,
    )
    .await
}

async fn execute_with(
    path: &Path,
    plan: &Authority<'_>,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
    dispatch: bool,
) -> Result<Vec<u8>> {
    let directory = crate::output_publication::prepare(path, 0o700)?;
    if path.join("reply.bin").try_exists().map_err(supplier)? {
        admit(
            &store::read(&path.join("signed.json"), store::MAX_RECORD)?,
            plan,
            call,
        )?;
        return store::read(&path.join("reply.bin"), 2 * 1024 * 1024);
    }
    if path.join("rejected.json").try_exists().map_err(supplier)? {
        return Err(Error::Blocked(format!(
            "retained rejection at {}; no replacement attempt",
            path.display()
        )));
    }
    let signed = path.join("signed.json");
    let bytes = if signed.try_exists().map_err(supplier)? {
        store::read(&signed, store::MAX_RECORD)?
    } else {
        // No dispatch can occur before signed.json is durably published. Scratch
        // from interrupted signing has no submission authority and can be rebuilt.
        for name in ["arguments.bin", "signing.json"] {
            let scratch = path.join(name);
            if scratch.try_exists().map_err(supplier)? {
                std::fs::remove_file(scratch).map_err(supplier)?;
            }
        }
        let bytes = sign(path, plan, call, identity, icp)?;
        store::put_in(&directory, std::ffi::OsStr::new("signed.json"), &bytes)?;
        bytes
    };
    let attempt = admit(&bytes, plan, call)?;
    if now()? >= attempt.update.expiry() {
        return Err(Error::Blocked(format!(
            "original signed attempt {} expired without a retained reply; do not re-sign or recreate",
            crate::hex(attempt.update.request_id())
        )));
    }
    // Publish exact bytes before this boundary. Retrying sends only this same
    // request ID within its original validity window; never a fresh signature.
    let result = if dispatch {
        Some(bounded(client.dispatch(&attempt.update, crate::network::context()?.1)).await)
    } else {
        None
    };
    if let Some(Ok(Dispatch::Observed(observation))) = result
        && let RequestStatus::Replied(reply) = observation.status()
    {
        store::put_in(&directory, std::ffi::OsStr::new("reply.bin"), reply)?;
        return Ok(reply.clone());
    }
    // Transport failure or acceptance is not effect failure. Read the certified
    // status for the original request using its retained signed read-state call.
    for _ in 0..30 {
        if now()? >= attempt.update.expiry() {
            break;
        }
        let observation = bounded(client.observe_prepared_request(
            &attempt.update,
            &attempt.status,
            crate::network::context()?.1,
        ))
        .await?;
        match observation.status() {
            RequestStatus::Replied(reply) => {
                store::put_in(&directory, std::ffi::OsStr::new("reply.bin"), reply)?;
                return Ok(reply.clone());
            }
            RequestStatus::Rejected { code, message, .. } => {
                store::put_json_in(
                    &directory,
                    std::ffi::OsStr::new("rejected.json"),
                    &Rejection {
                        code: code.to_string(),
                        message: message.clone(),
                    },
                )?;
                return Err(Error::Blocked(
                    "original request rejected; retained evidence requires review".into(),
                ));
            }
            RequestStatus::Done => break,
            _ => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
    Err(Error::Blocked(format!(
        "request {} has no retained terminal reply; resume this operation only",
        crate::hex(attempt.update.request_id())
    )))
}

fn admit_signer(icp: &Path) -> Result<()> {
    let version = crate::process::capture(
        Command::new(icp).arg("--version"),
        1024,
        1024,
        Duration::from_secs(5),
    )
    .map_err(supplier)?;
    if version != b"icp 1.7.0\n" {
        return Err(Error::Invalid(
            "signer must be repository-qualified ICP CLI 1.7.0; run make install-ic-tools".into(),
        ));
    }
    Ok(())
}

/// Read a public controller principal from the qualified signer. No keys or signed calls are exported.
/// # Errors
/// Refuses an unavailable/unqualified signer, missing identity or invalid principal output.
pub fn signing_principal(identity: &str, icp: &Path) -> Result<String> {
    admit_signer(icp)?;
    let output = crate::process::capture(
        Command::new(icp).args(["identity", "principal", "--identity", identity]),
        1024,
        4096,
        Duration::from_secs(30),
    )
    .map_err(supplier)?;
    crate::network::principal(std::str::from_utf8(&output).map_err(supplier)?.trim())
        .map(|principal| principal.to_text())
}
