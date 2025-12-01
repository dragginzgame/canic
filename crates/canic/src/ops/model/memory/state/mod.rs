mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::model::memory::state::{AppStateData, SubnetStateData};

/// DTOs exposed via ops-layer APIs.
pub type AppStateDto = AppStateData;
pub type SubnetStateDto = SubnetStateData;
