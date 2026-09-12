//! Store-local fixture admission under existing capacity and lifecycle owners.

use crate::{
    config,
    ops::{fixture_store, storage::template::WasmStoreGcOps},
};
use canic_core::{
    control_plane_support::workflow::runtime::fleet_activation::FleetActivationWorkflow,
    dto::{
        error::Error,
        fixture_provisioning::{
            FixtureChunkUpload, FixtureDescriptor, FixtureGrant, FixtureGrantRequest,
            FixtureSourceStatus, FixtureStoreError,
        },
    },
};

pub fn prepare(
    descriptor: FixtureDescriptor,
) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> {
    let maximum = writable_capacity()?;
    Ok(fixture_store::prepare(
        descriptor,
        maximum,
        fixture_store::template_bytes(),
    ))
}

pub fn upload(
    request: FixtureChunkUpload,
) -> Result<Result<FixtureSourceStatus, FixtureStoreError>, Error> {
    let maximum = writable_capacity()?;
    Ok(fixture_store::upload(
        request,
        maximum,
        fixture_store::template_bytes(),
    ))
}

pub fn set_grant(
    request: FixtureGrantRequest,
) -> Result<Result<FixtureGrant, FixtureStoreError>, Error> {
    let maximum = writable_capacity()?;
    let authority = FleetActivationWorkflow::wasm_store_authority().map_err(Error::from)?;
    Ok(fixture_store::set_grant(
        request,
        &authority,
        maximum,
        fixture_store::template_bytes(),
    ))
}

fn writable_capacity() -> Result<u64, Error> {
    WasmStoreGcOps::require_writable()?;
    config::current_wasm_store()
        .map(|config| config.max_store_bytes())
        .map_err(Error::from)
}
