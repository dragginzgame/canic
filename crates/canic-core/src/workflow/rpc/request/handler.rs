use crate::{
    Error, PublicError,
    dto::rpc::{Request, Response},
    ops::runtime::env::EnvOps,
    workflow::lifecycle::{create_canister_response, cycles_response, upgrade_canister_response},
};

/// Handle a root-bound orchestration request and produce a [`Response`].
pub(crate) async fn response_internal(req: Request) -> Result<Response, Error> {
    EnvOps::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}

pub async fn response(req: Request) -> Result<Response, PublicError> {
    response_internal(req).await.map_err(PublicError::from)
}
