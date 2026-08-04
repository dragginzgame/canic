#![expect(clippy::unused_async)]

use candid::{CandidType, Deserialize, Principal};
use canic::{Error, api::canister::placement::ScalingApi, prelude::*};

const WENZELROLL_POOL: &str = "wenzelrolls";

#[derive(CandidType, Clone, Debug, Deserialize)]
struct WenzelrollView {
    canister_id: Principal,
    created_at_secs: u64,
    url: String,
}

canic::start!();

async fn canic_setup() {}
async fn canic_install(_: Option<Vec<u8>>) {}
async fn canic_upgrade() {}

#[canic_query(public)]
fn wenzelrolls() -> Result<Vec<WenzelrollView>, Error> {
    let mut canisters = ScalingApi::registry()
        .0
        .into_iter()
        .filter(|canister| canister.entry.pool == WENZELROLL_POOL)
        .map(|canister| WenzelrollView {
            canister_id: canister.pid,
            created_at_secs: canister.entry.created_at_secs,
            url: format!("https://{}.raw.icp.net/", canister.pid),
        })
        .collect::<Vec<_>>();
    canisters.sort_by(|left, right| {
        left.created_at_secs
            .cmp(&right.created_at_secs)
            .then_with(|| {
                left.canister_id
                    .as_slice()
                    .cmp(right.canister_id.as_slice())
            })
    });
    Ok(canisters)
}

#[canic_query(public)]
fn wenzelroll_can_create() -> Result<bool, Error> {
    ScalingApi::plan_create_worker(WENZELROLL_POOL)
}

#[canic_update(requires(caller::is_whitelisted()))]
async fn wenzelroll_create() -> Result<WenzelrollView, Error> {
    let canister_id = ScalingApi::create_worker(WENZELROLL_POOL).await?;
    let created_at_secs = ScalingApi::registry()
        .0
        .into_iter()
        .find(|site| site.pid == canister_id)
        .map_or(0, |site| site.entry.created_at_secs);

    Ok(WenzelrollView {
        canister_id,
        created_at_secs,
        url: format!("https://{canister_id}.raw.icp.net/"),
    })
}

canic::finish!();
