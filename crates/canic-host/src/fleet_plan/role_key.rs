//! Resolve the one issuer output authorized by a receiver-enabled fleet plan.
use super::{FleetPlan, Receipt, invalid};
use crate::{Result, network::principal, operation, signed, store, supplier};
use std::path::Path;

pub(crate) const METHOD: &str = "prepare_role_key";
fn frame(direction: ic_wire::Direction) -> ic_wire::Encoding {
    ic_wire::Encoding::Frame {
        method: ic_wire::method_id(METHOD),
        direction,
    }
}

pub(crate) fn request() -> Result<Vec<u8>> {
    ic_wire::encode(
        &(),
        frame(ic_wire::Direction::Request),
        canic_management_protocol::LIMITS,
    )
    .map_err(supplier)
}

fn call(root: &Receipt) -> Result<signed::Call<'static>> {
    let id = principal(&root.canister)?;
    Ok(signed::Call {
        canister: id,
        effective: id,
        method: METHOD,
        args: request()?,
    })
}

pub(super) fn retained(dir: &Path, fleet: &FleetPlan, root: &Receipt) -> Result<Vec<u8>> {
    fleet.verify_root_receipt(root)?;
    let path = dir.join("role-key");
    signed::validate_retained(&path, &fleet.root, &call(root)?)?;
    decode(&store::read(&path.join("reply.bin"), 512)?).map(Vec::from)
}

pub(crate) fn decode(reply: &[u8]) -> Result<[u8; 32]> {
    let key: std::result::Result<ic_wire::Bytes<32>, String> = ic_wire::decode(
        reply,
        frame(ic_wire::Direction::Response),
        canic_management_protocol::LIMITS,
    )
    .map_err(supplier)?;
    let key = key.map_err(crate::Error::Blocked)?.into_vec();
    key.try_into().map_err(|_| invalid())
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
