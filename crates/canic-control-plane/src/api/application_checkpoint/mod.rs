//! Root checkpoint facade for controller-authenticated endpoints.
//!
//! Workflow owns discovery and bounded remote work; ops owns durable transitions.

use canic_contracts::dto::{
    application_checkpoint::{
        RootCheckpointBeginRequest, RootCheckpointRequest, RootCheckpointStatus,
    },
    error::Error,
};

/// Root-owned checkpoint coordination without snapshot or spending authority.
pub struct RootCheckpointApi;

impl RootCheckpointApi {
    pub fn begin(request: RootCheckpointBeginRequest) -> Result<RootCheckpointStatus, Error> {
        crate::workflow::application_checkpoint::begin(request)
    }
    pub fn status() -> Result<Option<RootCheckpointStatus>, Error> {
        crate::workflow::application_checkpoint::status()
    }
    pub async fn advance(request: RootCheckpointRequest) -> Result<RootCheckpointStatus, Error> {
        crate::workflow::application_checkpoint::advance(request).await
    }
    pub fn release(request: RootCheckpointRequest) -> Result<RootCheckpointStatus, Error> {
        crate::workflow::application_checkpoint::release(request)
    }
}
