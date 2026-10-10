//! Runtime-owned intent identities and private imports of the shared vocabulary.

mod intent;
pub(crate) use canic_contracts::ids::*;
pub use intent::{IntentId, IntentResourceKey};
