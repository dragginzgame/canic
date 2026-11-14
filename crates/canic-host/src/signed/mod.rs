//! ICP CLI owns keys and signing; Canic owns exact-attempt custody and submission.

#[cfg(test)]
mod tests;

use crate::{
    Error, Plan, Result,
    network::{bounded, principal},
    now, store, supplier,
};
use base64::{Engine, prelude::BASE64_STANDARD};
use ic_platform::{
    agent::{Agent, CallResponse, EffectiveId, RequestId, RequestStatusResponse, to_request_id},
    ic_protocol::{self, EnvelopeContent},
    principal::Principal,
};
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
    envelope: Vec<u8>,
    status_check: Vec<u8>,
    id: RequestId,
    expiry: u64,
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
    let content =
        ic_protocol::decode_call_content_strict(&envelope, store::MAX_RECORD).map_err(supplier)?;
    let id = to_request_id(&content).map_err(supplier)?;
    let expiry = match content {
        EnvelopeContent::Call {
            sender,
            canister_id,
            method_name,
            arg,
            ingress_expiry,
            sender_info,
            ..
        } if sender.as_slice() == principal(plan.controller)?.as_slice()
            && canister_id.as_slice() == call.canister.as_slice()
            && method_name == call.method
            && arg == call.args
            && sender_info.is_none()
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
    if crate::hex(id.as_slice()) != message.request.request_id {
        return Err(Error::Invalid("signed request ID mismatch".into()));
    }
    let status_check = BASE64_STANDARD
        .decode(&message.request.status_check)
        .map_err(supplier)?;
    let status =
        ic_protocol::decode_envelope(&status_check, store::MAX_RECORD).map_err(supplier)?;
    match status.content.as_ref() {
        EnvelopeContent::ReadState {
            sender,
            ingress_expiry,
            paths,
        } if sender.as_slice() == principal(plan.controller)?.as_slice()
            && *ingress_expiry == expiry
            && paths.len() == 1
            && paths[0].len() == 2
            && paths[0][0].as_ref() == b"request_status"
            && paths[0][1].as_ref() == id.as_slice() => {}
        _ => {
            return Err(Error::Invalid(
                "signed status check does not bind the same attempt".into(),
            ));
        }
    }
    Ok(Attempt {
        envelope,
        status_check,
        id,
        expiry,
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
    let version = ic_host_process::tool::capture_group_command(
        Command::new(icp).arg("--version"),
        ic_host_process::tool::OutputLimits {
            stdout: ic_host_process::tool::OutputLimit::Terminate(1024),
            stderr: ic_host_process::tool::OutputLimit::Terminate(1024),
            timeout: Some(Duration::from_secs(5)),
        },
    )
    .map_err(supplier)?
    .require_success()
    .map_err(supplier)?;
    if version.stdout != b"icp 1.7.0\n" {
        return Err(Error::Invalid(
            "signer must be repository-qualified ICP CLI 1.7.0; run make install-ic-tools".into(),
        ));
    }
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
    agent: &Agent,
) -> Result<Vec<u8>> {
    execute_with(path, &Authority::from(plan), call, identity, icp, agent).await
}

/// Only the fixed Root observation method receives fresh read authority after plan expiry.
pub(crate) async fn observe(
    path: &Path,
    plan: &Plan,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<Vec<u8>> {
    let owner = plan
        .management
        .as_ref()
        .ok_or_else(|| Error::Invalid("observation needs Root binding".into()))?;
    let args = ic_wire::native::encode_request(
        canic_management_protocol::STATUS,
        &(),
        canic_management_protocol::LIMITS,
    )
    .map_err(supplier)?;
    if call.canister != principal(&owner.root)?
        || call.effective != call.canister
        || call.method != "observe"
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
        agent,
    )
    .await
}

async fn execute_with(
    path: &Path,
    plan: &Authority<'_>,
    call: &Call<'_>,
    identity: &str,
    icp: &Path,
    agent: &Agent,
) -> Result<Vec<u8>> {
    ic_host_fs::durable::prepare_private_parents(path).map_err(supplier)?;
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
        store::put(&signed, &bytes)?;
        bytes
    };
    let attempt = admit(&bytes, plan, call)?;
    if now()? >= attempt.expiry {
        return Err(Error::Blocked(format!(
            "original signed attempt {} expired without a retained reply; do not re-sign or recreate",
            attempt.id
        )));
    }
    // Publish exact bytes before this boundary. Retrying sends only this same
    // request ID within its original validity window; never a fresh signature.
    let effective = EffectiveId::Canister(call.effective.to_text().parse().map_err(supplier)?);
    let result = bounded(agent.update_signed(effective, attempt.envelope)).await;
    if let Ok(CallResponse::Response(reply)) = result {
        store::put(&path.join("reply.bin"), &reply)?;
        return Ok(reply);
    }
    // Transport failure or acceptance is not effect failure. Read the certified
    // status for the original request using its retained signed read-state call.
    for _ in 0..30 {
        if now()? >= attempt.expiry {
            break;
        }
        let (status, _) = bounded(agent.request_status_signed(
            &attempt.id,
            effective,
            attempt.status_check.clone(),
        ))
        .await?;
        match status {
            RequestStatusResponse::Replied(reply) => {
                store::put(&path.join("reply.bin"), &reply.arg)?;
                return Ok(reply.arg);
            }
            RequestStatusResponse::Rejected(rejection) => {
                store::put_json(
                    &path.join("rejected.json"),
                    &Rejection {
                        code: format!("{:?}", rejection.reject_code),
                        message: rejection.reject_message,
                    },
                )?;
                return Err(Error::Blocked(
                    "original request rejected; retained evidence requires review".into(),
                ));
            }
            RequestStatusResponse::Done => break,
            _ => tokio::time::sleep(Duration::from_secs(1)).await,
        }
    }
    Err(Error::Blocked(format!(
        "request {} has no retained terminal reply; resume this operation only",
        attempt.id
    )))
}
