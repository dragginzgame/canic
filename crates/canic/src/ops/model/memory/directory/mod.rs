mod app;
mod subnet;

pub use app::*;
pub use subnet::*;

use crate::{model::memory::directory::PrincipalList, types::CanisterType};
use candid::CandidType;
use serde::Serialize;

///
/// DirectoryView
/// DTO wrapper for directory exports.
///

pub type DirectoryView = Vec<(CanisterType, PrincipalList)>;

///
/// DirectoryPageDto
///

#[derive(CandidType, Serialize)]
pub struct DirectoryPageDto {
    pub entries: DirectoryView,
    pub total: u64,
}

// paginate
// shared between both app and subnet
#[allow(clippy::cast_possible_truncation)]
fn paginate(view: DirectoryView, offset: u64, limit: u64) -> DirectoryView {
    let start = offset as usize;
    let len = limit as usize;

    view.into_iter().skip(start).take(len).collect()
}
