use crate::{
    Error,
    access::env,
    dto::rpc::{Request, Response},
    workflow::lifecycle::{create_canister_response, cycles_response, upgrade_canister_response},
};

/// Handle a root-bound orchestration request and produce a [`Response`].
pub async fn response(req: Request) -> Result<Response, Error> {
    env::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}
