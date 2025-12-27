use crate::ids::CanisterRole;
use candid::Principal;

///
/// DirectoryView
///

pub type DirectoryView = Vec<(CanisterRole, Principal)>;
