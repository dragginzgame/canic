//! Controller-authenticated adapters for exact retained-operation retry windows.
use canic_contracts::dto::{
    error::Error,
    operation_retry::{Operation, ResumeRequest, Status},
};
/// Endpoint adapter; workflow retains resumption and timer-owner coordination.
pub struct OperationRetryApi;
impl OperationRetryApi {
    pub fn status(operation: Operation) -> Result<Status, Error> {
        crate::workflow::continuation_retry::status(operation).map_err(Into::into)
    }
    pub fn resume(request: ResumeRequest) -> Result<Status, Error> {
        crate::workflow::continuation_retry::resume(request).map_err(Into::into)
    }
}
