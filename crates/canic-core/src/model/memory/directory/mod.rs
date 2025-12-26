pub mod app;
pub mod subnet;

pub use app::AppDirectory;
pub use subnet::SubnetDirectory;

use crate::ids::CanisterRole;
use candid::Principal;

///
/// DirectoryView
///

pub type DirectoryView = Vec<(CanisterRole, Principal)>;
