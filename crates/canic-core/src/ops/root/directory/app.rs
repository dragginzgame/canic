// ops/root/app_directory.rs

use crate::{
    model::memory::directory::DirectoryView, ops::storage::directory::build_from_registry,
};

///
/// Root-only directory builder.
/// This code must never be linked into non-root canisters.
///

pub struct RootAppDirectoryBuilder;

impl RootAppDirectoryBuilder {
    pub fn build_from_registry() -> DirectoryView {
        build_from_registry()
    }
}
