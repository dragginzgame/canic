use crate::{
    dto::page::{Page, PageRequest},
    ids::CanisterRole,
    model::memory::directory::{SubnetDirectory, SubnetDirectoryData},
    ops::view::paginate_vec,
};
use candid::Principal;

///
/// SubnetDirectoryOps
///

pub struct SubnetDirectoryOps;

impl SubnetDirectoryOps {
    #[must_use]
    pub fn get(role: &CanisterRole) -> Option<Principal> {
        SubnetDirectory::export()
            .iter()
            .find_map(|(t, pid)| (t == role).then_some(*pid))
    }

    #[must_use]
    pub fn page(request: PageRequest) -> Page<(CanisterRole, Principal)> {
        paginate_vec(SubnetDirectory::export(), request)
    }

    #[must_use]
    pub fn export() -> SubnetDirectoryData {
        SubnetDirectory::export()
    }

    pub fn import(data: SubnetDirectoryData) {
        SubnetDirectory::import(data);
    }
}
