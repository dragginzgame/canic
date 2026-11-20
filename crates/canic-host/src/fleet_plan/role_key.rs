//! Resolve the one issuer output authorized by a receiver-enabled fleet plan.
use super::{FleetPlan, Receipt, invalid};
use crate::{Result, network::principal, operation, signed, store, supplier};
use std::path::Path;

const METHOD: &str = "prepare_role_key";
fn frame(direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame {
        method: ic_wire::method_id(METHOD),
        direction,
    }
}

fn call(root: &Receipt) -> Result<signed::Call<'static>> {
    let id = principal(&root.canister)?;
    Ok(signed::Call {
        canister: id,
        effective: id,
        method: METHOD,
        args: ic_wire::encode(
            &(),
            frame(ic_wire::Direction::Request),
            canic_management_protocol::LIMITS,
        )
        .map_err(supplier)?,
    })
}

pub(super) fn retained(dir: &Path, fleet: &FleetPlan, root: &Receipt) -> Result<Vec<u8>> {
    fleet.verify_root_receipt(root)?;
    let path = dir.join("role-key");
    signed::validate_retained(&path, &fleet.root, &call(root)?)?;
    let key: std::result::Result<ic_wire::Bytes<32>, String> = ic_wire::decode(
        &store::read(&path.join("reply.bin"), 512)?,
        frame(ic_wire::Direction::Response),
        canic_management_protocol::LIMITS,
    )
    .map_err(supplier)?;
    let key = key.map_err(crate::Error::Blocked)?.into_vec();
    if key.len() != 32 {
        return Err(invalid());
    }
    Ok(key)
}

pub(super) async fn prepare(
    dir: &Path,
    fleet: &FleetPlan,
    root: &Receipt,
    identity: &str,
    icp: &Path,
) -> Result<()> {
    fleet.verify_root_receipt(root)?;
    let path = dir.join("role-key");
    if !path.join("reply.bin").try_exists().map_err(supplier)? {
        let client = fleet.root.network.client()?;
        let call = call(root)?;
        operation::controllers(&client, call.canister, &fleet.root.controller).await?;
        if operation::module(&client, call.canister).await? != fleet.root.artifact.sha256 {
            return Err(invalid());
        }
        signed::execute(&path, &fleet.root, &call, identity, icp, &client).await?;
    }
    retained(dir, fleet, root).map(drop)
}
