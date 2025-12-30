/// Cross-canister RPC method names used by Canic internal calls.
///
/// These string values act as a wire protocol. Centralizing them makes it harder to
/// accidentally drift between caller and callee when refactoring.

pub const CANIC_RESPONSE: &str = "canic_response";
pub const CANIC_SYNC_STATE: &str = "canic_sync_state";
pub const CANIC_SYNC_TOPOLOGY: &str = "canic_sync_topology";
