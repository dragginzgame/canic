/// Handle a root-bound orchestration request and produce a [`Response`].
pub async fn response(req: Request) -> Result<Response, Error> {
    OpsError::require_root()?;

    match req {
        Request::CreateCanister(req) => create_canister_response(&req).await,
        Request::UpgradeCanister(req) => upgrade_canister_response(&req).await,
        Request::Cycles(req) => cycles_response(&req).await,
    }
}
