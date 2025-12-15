//! Stable-memory state adapters.

mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

pub use crate::model::memory::state::{AppStateData, SubnetStateData};
