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
            "revoke_auth"
        } else {
            "configure_auth"
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
    if let Some(auth) = receipt(dir, plan)? {
        return Ok(crate::ApplyOutcome::Auth { auth });
    }
    let path = dir.join("auth");
    let prepared = path.join("signed.json").try_exists().map_err(supplier)?;
    if reconcile && !prepared {
        return Err(Error::Blocked(
            "Auth reconciliation needs the original signed attempt; use apply first".into(),
        ));
    }
    let agent = plan.network.agent()?;
    {
        let approval = plan.auth().expect("Auth caller");
        for (target, controller) in [
            (&approval.root, plan.controller.as_str()),
            (&approval.receiver, approval.root.canister.as_str()),
        ] {
            let id = crate::network::principal(&target.canister)?;
            operation::controllers(&agent, id, controller).await?;
            if operation::module(&agent, id).await? != target.module {
                return Err(Error::Blocked(
                    "Auth installed code differs from approval".into(),
                ));
            }
        }
    }
    signed::execute(&path, plan, &call(plan)?, identity, icp, &agent).await?;
    let auth = receipt(dir, plan)?
        .ok_or_else(|| Error::Blocked("Auth has no retained response".into()))?;
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
