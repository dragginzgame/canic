//! Store handover and readiness extend the existing direct deployment owner.

#[cfg(test)]
mod tests;

use crate::{
    DeploymentName, Error, Plan, PlanAction, PlanInput, Result, SelectedFleet, UpgradeTarget,
    artifact,
    network::{principal, subnet},
    operation, plan, signed, store, supplier,
};
use canic_management_protocol::publication as wire;
use ic_platform::native::ReadClient;
use ic_platform_protocol::management as mgmt;
use ic_platform_types::Principal;
use ic_wasm_store_protocol::Info;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub(crate) const OBSERVATIONS: usize = 32;

/// Create a separately controlled Store under an explicitly bound existing Root.
/// Uses direct creation/install custody, then retained controller handover and readiness.
/// # Errors
/// Refuses occupied slots, non-Store roles, missing Root identity or unrelated effect authority.
pub fn create_store_plan(
    journal: &Path,
    wasm: &Path,
    input: PlanInput,
    selection: &SelectedFleet,
    canister: &str,
    root_module: &str,
) -> Result<(String, Plan)> {
    selection.validate()?;
    let slot = selection
        .config
        .canisters
        .get(canister)
        .ok_or_else(|| Error::Invalid("unknown Store slot".into()))?;
    let root = slot
        .parent
        .as_ref()
        .and_then(|p| selection.config.canisters.get(p))
        .ok_or_else(|| Error::Invalid("Store requires a declared Root parent".into()))?;
    if slot.role.as_deref() != Some("wasm-store")
        || slot.id.is_some()
        || root.role.as_deref() != Some("root")
        || root.parent.is_some()
        || input.network != selection.config.network
        || input.target.is_some()
        || input.previous_module.is_some()
    {
        return Err(Error::Invalid(
            "Store bootstrap requires an unbound wasm-store under a bound top-level Root".into(),
        ));
    }
    let root = UpgradeTarget {
        canister: root.id.clone().ok_or_else(|| {
            Error::Invalid("bind the existing Root before Store bootstrap".into())
        })?,
        module: root_module.into(),
    };
    let bytes = store::read(wasm, artifact::MAX_WASM)?;
    let now = crate::now()?;
    let plan = Plan {
        version: 9,
        network: input.network,
        controller: input.controller,
        artifact: artifact::admit(&bytes)?,
        action: PlanAction::CreateStore {
            cycles: input.creation_cycles,
            root,
        },
        created_at_ns: now,
        expires_at_ns: now + 3_600_000_000_000,
        deployment: Some(DeploymentName {
            app: selection.app.clone(),
            fleet: selection.fleet.clone(),
            canister: canister.into(),
        }),
    };
    plan::retain_locked(&store::Store::open(journal)?, plan, &bytes)
}

pub(crate) async fn verify_root(client: &ReadClient, plan: &Plan) -> Result<()> {
    if let Some(root) = plan.store_root() {
        let id = principal(&root.canister)?;
        operation::controllers(client, id, &plan.controller).await?;
        if operation::module(client, id).await? != root.module {
            return Err(Error::Blocked(
                "Store bootstrap Root code differs from approval".into(),
            ));
        }
    }
    Ok(())
}

pub(crate) fn placement(dir: &Path, plan: &Plan) -> Result<Option<Principal>> {
    if plan.store_root().is_none() {
        return Ok(None);
    }
    let id: String = store::json(&dir.join("store-subnet.json"))?;
    Ok(Some(principal(&id)?))
}

/// Store creation means colocating with the approved Root, never arbitrary CMC placement.
pub(crate) async fn prepare_placement(dir: &Path, plan: &Plan, client: &ReadClient) -> Result<()> {
    let Some(root) = plan.store_root() else {
        return Ok(());
    };
    let current = subnet(client, &root.canister).await?;
    let path = dir.join("store-subnet.json");
    if path.try_exists().map_err(supplier)? {
        if placement(dir, plan)? != Some(current) {
            return Err(Error::Blocked(
                "Root subnet differs from retained Store placement".into(),
            ));
        }
    } else {
        if dir
            .join("create/signed.json")
            .try_exists()
            .map_err(supplier)?
        {
            return Err(Error::Invalid(
                "Store creation lost its retained subnet binding".into(),
            ));
        }
        store::put_json(&path, &current.to_text())?;
    }
    Ok(())
}

pub(crate) async fn verify_placement(
    dir: &Path,
    plan: &Plan,
    canister: Principal,
    client: &ReadClient,
) -> Result<()> {
    if plan.store_root().is_some()
        && placement(dir, plan)? != Some(subnet(client, &canister.to_text()).await?)
    {
        return Err(Error::Blocked(
            "created Store is not on the approved Root subnet".into(),
        ));
    }
    Ok(())
}

pub(crate) fn handover(plan: &Plan, canister: Principal) -> Result<signed::Call<'static>> {
    let root = plan
        .store_root()
        .ok_or_else(|| Error::Invalid("Store bootstrap plan required".into()))?;
    Ok(signed::Call {
        canister: Principal::management_canister(),
        effective: canister,
        method: "update_settings",
        // Keep the complete settings schema bound into existing signed attempts.
        args: mgmt::UpdateSettingsArgs {
            canister_id: canister,
            settings: mgmt::CanisterSettings {
                controllers: Some(vec![root.canister.parse().map_err(supplier)?]),
                ..Default::default()
            },
            sender_canister_version: None,
        }
        .encode(4096)
        .map_err(Error::Encode)?,
    })
}

pub(crate) fn observation(
    plan: &Plan,
    canister: Principal,
    deadline: u64,
) -> Result<signed::Call<'static>> {
    let root = plan
        .store_root()
        .ok_or_else(|| Error::Invalid("Store bootstrap plan required".into()))?;
    let id = principal(&root.canister)?;
    let request = wire::InfoRequest {
        store: canister.to_text(),
        expires_at_ns: deadline,
    };
    let frame = ic_wire::Encoding::Frame {
        method: if plan.version == 9 {
            wire::OBSERVATION
        } else {
            wire::INFO
        },
        direction: ic_wire::Direction::Request,
    };
    let args = if plan.version == 9 {
        ic_wire::encode(
            &wire::ObservationRequest::Info(request),
            frame,
            canic_management_protocol::LIMITS,
        )
    } else {
        ic_wire::encode(&request, frame, canic_management_protocol::LIMITS)
    }
    .map_err(supplier)?;
    Ok(signed::Call {
        canister: id,
        effective: id,
        method: "observe_store",
        args,
    })
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Readiness {
    observation: usize,
    info: Info,
}

fn decode(plan: &Plan, bytes: &[u8]) -> Result<Info> {
    let frame = ic_wire::Encoding::Frame {
        method: if plan.version == 9 {
            wire::OBSERVATION
        } else {
            wire::INFO
        },
        direction: ic_wire::Direction::Response,
    };
    let result: wire::InfoReply = if plan.version == 9 {
        ic_wire::decode::<wire::ObservationReply>(bytes, frame, wire::RESPONSE_LIMITS)
            .map_err(supplier)?
            .and_then(|v| match v {
                wire::Observation::Info(info) => Ok(info),
                wire::Observation::Custody(_) => Err(wire::Error::Uncertain),
            })
    } else {
        ic_wire::decode(bytes, frame, wire::RESPONSE_LIMITS).map_err(supplier)?
    };
    result.map_err(|error| {
        Error::Blocked(format!(
            "Store readiness returned {error:?}; retain evidence and reconcile"
        ))
    })
}

/// Offline receipt admission still requires its original signed readiness observation.
pub(crate) fn retained(dir: &Path, plan: &Plan, canister: Principal) -> Result<()> {
    let value: Readiness = store::json(&dir.join("store-ready.json"))?;
    if value.observation >= OBSERVATIONS {
        return Err(Error::Invalid("Store observation budget exceeded".into()));
    }
    let reply = signed::retained_store(
        &dir.join(format!("store-observe/{}", value.observation)),
        plan,
        canister,
    )?;
    if decode(plan, &reply)? != value.info {
        return Err(Error::Invalid(
            "Store readiness differs from retained observation".into(),
        ));
    }
    Ok(())
}

/// The caller has retained creation and installation evidence under the same journal lock.
pub(crate) async fn finish(
    dir: &Path,
    plan: &Plan,
    canister: Principal,
    identity: &str,
    icp: &Path,
    client: &ReadClient,
    read_only: bool,
) -> Result<()> {
    verify_placement(dir, plan, canister, client).await?;
    let path = dir.join("handover");
    let call = handover(plan, canister)?;
    let prepared = path.join("signed.json").try_exists().map_err(supplier)?;
    if prepared {
        signed::validate_retained(&path, plan, &call)?;
    }
    if !prepared
        || operation::controllers(client, canister, plan.application_controller())
            .await
            .is_err()
    {
        if read_only {
            return Err(Error::Blocked(
                "Store handover is not established; reconcile grants no mutation authority".into(),
            ));
        }
        operation::controllers(client, canister, &plan.controller).await?;
        // Module and Root binding are checked before surrendering operator control.
        if operation::module(client, canister).await? != plan.artifact.sha256 {
            return Err(Error::Blocked("Store code differs before handover".into()));
        }
        verify_root(client, plan).await?;
        signed::execute(&path, plan, &call, identity, icp, client).await?;
    }
    operation::controllers(client, canister, plan.application_controller()).await?;
    if operation::module(client, canister).await? != plan.artifact.sha256 {
        return Err(Error::Blocked("Store code differs after handover".into()));
    }
    if dir
        .join("store-ready.json")
        .try_exists()
        .map_err(supplier)?
    {
        return retained(dir, plan, canister);
    }
    for index in 0..OBSERVATIONS {
        let path = dir.join(format!("store-observe/{index}"));
        if path.join("reply.bin").try_exists().map_err(supplier)?
            || path.join("rejected.json").try_exists().map_err(supplier)?
        {
            continue;
        }
        let reply = signed::observe_store(&path, plan, canister, identity, icp, client).await?;
        let info = decode(plan, &reply)?;
        store::put_json(
            &dir.join("store-ready.json"),
            &Readiness {
                observation: index,
                info,
            },
        )?;
        return Ok(());
    }
    Err(Error::Blocked(
        "Store readiness observation budget exhausted; retain evidence".into(),
    ))
}
