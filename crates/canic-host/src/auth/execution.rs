use super::{AuthReceipt, wire};
use crate::{Error, Plan, Result, operation, signed, store, supplier};
use std::path::Path;

fn call(plan: &Plan) -> Result<signed::Call<'static>> {
    let approval = plan
        .auth()
        .ok_or_else(|| Error::Invalid("Auth plan required".into()))?;
    let root = crate::network::principal(&approval.root.canister)?;
    Ok(signed::Call {
        canister: root,
        effective: root,
        method: if approval.revoke {
            "revoke_auth_access"
        } else {
            "grant_auth_access"
        },
        args: ic_wire::encode(
            &approval.grant,
            ic_wire::Encoding::Frame {
                method: if approval.revoke {
                    wire::REVOKE
                } else {
                    wire::CONFIGURE
                },
                direction: ic_wire::Direction::Request,
            },
            wire::LIMITS,
        )
        .map_err(supplier)?,
    })
}

pub(super) fn receipt(dir: &Path, plan: &Plan) -> Result<Option<AuthReceipt>> {
    let path = dir.join("auth");
    if !path.join("reply.bin").try_exists().map_err(supplier)? {
        return Ok(None);
    }
    signed::validate_retained(&path, plan, &call(plan)?)?;
    let revoked = plan.auth().expect("Auth caller").revoke;
    let reply: std::result::Result<(), wire::Error> = ic_wire::decode(
        &store::read(&path.join("reply.bin"), wire::LIMITS.bytes)?,
        ic_wire::Encoding::Frame {
            method: if revoked {
                wire::REVOKE
            } else {
                wire::CONFIGURE
            },
            direction: ic_wire::Direction::Response,
        },
        wire::LIMITS,
    )
    .map_err(supplier)?;
    reply
        .map_err(|e| Error::Blocked(format!("Auth returned {e:?}; original response retained")))?;
    Ok(Some(AuthReceipt {
        operation: plan.digest()?,
        revoked,
    }))
}

pub(crate) async fn apply(
    dir: &Path,
    plan: &Plan,
    identity: &str,
    icp: &Path,
    reconcile: bool,
) -> Result<crate::ApplyOutcome> {
    let path = dir.join("auth");
    let auth = signed::RetainedAttempt {
        plan,
        identity,
        icp,
        reconcile,
    }
    .execute(
        &path,
        "Auth reconciliation needs the original signed attempt; use apply first",
        "Auth has no retained response",
        || receipt(dir, plan),
        async |client| {
            let approval = plan.auth().expect("Auth caller");
            for (target, controller) in [
                (&approval.root, plan.controller.as_str()),
                (
                    &approval.receiver,
                    approval.expected_receiver_controller(&plan.controller),
                ),
            ] {
                let id = crate::network::principal(&target.canister)?;
                operation::controllers(client, id, controller).await?;
                if operation::module(client, id).await? != target.module {
                    return Err(Error::Blocked(
                        "Auth installed code differs from approval".into(),
                    ));
                }
            }
            call(plan)
        },
    )
    .await?;
    Ok(crate::ApplyOutcome::Auth { auth })
}

pub(crate) fn status(dir: &Path, plan: &Plan) -> Result<crate::CanisterStatus> {
    Ok(crate::CanisterStatus {
        operation: plan.digest()?,
        plan: plan.clone(),
        effects: std::collections::BTreeMap::from([(
            "auth".into(),
            operation::attempt_state(&dir.join("auth"))?.into(),
        )]),
        receipt: None,
        management: None,
        detail: crate::StatusDetail::Auth {
            auth: receipt(dir, plan)?,
        },
    })
}
