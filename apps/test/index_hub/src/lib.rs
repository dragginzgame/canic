#![expect(clippy::unused_async)]

use canic::{Error, api::canister::placement::PlacementIndexApi, prelude::*};

const POOL_NAME: &str = "items";

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

/// Resolve one indexed key, creating its exact managed child when absent.
#[canic_update(requires(env::build_local_only()))]
async fn resolve_item(
    key: String,
) -> Result<canic::dto::placement::index::PlacementIndexStatusResponse, Error> {
    PlacementIndexApi::resolve_or_create(POOL_NAME, key).await
}

canic::finish!();
