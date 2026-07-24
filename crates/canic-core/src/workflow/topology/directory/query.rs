//! Module: workflow::topology::directory::query
//!
//! Responsibility: expose read-only Fleet and Subnet Directory pages and role lookups.
//! Does not own: Directory storage mutation, endpoint authorization, or DTO schemas.
//! Boundary: workflow query facade over Directory storage ops.

use crate::{
    InternalError,
    cdk::types::Principal,
    dto::{
        page::{Page, PageRequest},
        topology::{
            DirectoryEntryResponse, FleetDirectoryPageResponse, SubnetDirectoryPageResponse,
        },
    },
    ids::CanisterRole,
    ops::{
        storage::directory::{
            fleet::FleetDirectoryOps, mapper::DirectoryEntryMapper, subnet::SubnetDirectoryOps,
        },
        topology::directory::current_provenance,
    },
    view::topology::DirectoryEntryView,
    workflow::view::paginate::paginate_vec,
};

///
/// FleetDirectoryQuery
///

pub struct FleetDirectoryQuery;

impl FleetDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        FleetDirectoryOps::get(&role)
    }

    pub fn page(page: PageRequest) -> Result<FleetDirectoryPageResponse, InternalError> {
        let page = directory_page(FleetDirectoryOps::entry_projections(), page);
        Ok(FleetDirectoryPageResponse {
            provenance: current_provenance()?,
            entries: page.entries,
            total: page.total,
        })
    }
}

///
/// SubnetDirectoryQuery
///

pub struct SubnetDirectoryQuery;

impl SubnetDirectoryQuery {
    #[must_use]
    pub fn get(role: CanisterRole) -> Option<Principal> {
        SubnetDirectoryOps::get(&role)
    }

    pub fn page(page: PageRequest) -> Result<SubnetDirectoryPageResponse, InternalError> {
        let page = directory_page(SubnetDirectoryOps::entry_projections(), page);
        Ok(SubnetDirectoryPageResponse {
            provenance: current_provenance()?,
            entries: page.entries,
            total: page.total,
        })
    }
}

// Paginate read-only Directory projections and let ops own response mapping.
fn directory_page(
    entries: Vec<DirectoryEntryView>,
    page: PageRequest,
) -> Page<DirectoryEntryResponse> {
    DirectoryEntryMapper::projection_page_to_response(paginate_vec(entries, page))
}
