//! Env operations facade.

use crate::model::memory::Env;

pub use crate::model::memory::env::EnvData;

///
/// EnvOps
///
pub struct EnvOps;

impl EnvOps {
    /// Export a snapshot of the current environment metadata.
    #[must_use]
    pub fn export() -> EnvData {
        Env::export()
    }
}
