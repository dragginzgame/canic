use crate::{
    dto::{
        error::Error,
        icp_refill::{IcpRefillEndpointResponse, IcpRefillRequest},
    },
    workflow::ic::icp_refill::IcpRefillWorkflow,
};

///
/// IcpRefillApi
///

pub struct IcpRefillApi;

impl IcpRefillApi {
    pub async fn refill(request: IcpRefillRequest) -> Result<IcpRefillEndpointResponse, Error> {
        if request.dry_run {
            return IcpRefillWorkflow::dry_run_manual_refill(request)
                .await
                .map(IcpRefillEndpointResponse::DryRun)
                .map_err(Error::from);
        }

        IcpRefillWorkflow::execute_manual_refill(request)
            .await
            .map(IcpRefillEndpointResponse::Refill)
            .map_err(Error::from)
    }
}
