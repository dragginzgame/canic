use crate::{PublicError, dto::state::AppCommand, workflow};

///
/// Apply an application-level command.
///
/// Public API entry point for mutating application state. This function:
/// - is safe to call from user canisters and endpoint code
/// - returns [`PublicError`] suitable for IC boundaries
/// - delegates all orchestration to the internal workflow layer
///
/// Layering:
///     user canister → api → workflow
///
pub async fn apply_command(cmd: AppCommand) -> Result<(), PublicError> {
    workflow::app::apply_command(cmd)
        .await
        .map_err(PublicError::from)
}
