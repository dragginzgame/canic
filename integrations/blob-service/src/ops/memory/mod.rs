//! Module: blob_service::ops::memory
//!
//! Responsibility: register application grants and open the existing default runtime.
//! Does not own: memory bootstrap, placement policy or storage schemas.
//! Boundary: Canic bootstraps once before the lifecycle participant opens the grants.

use ic_blob_storage::ops::service::{
    installation::{self, INSTALLATION_MEMORY_KEY},
    stores::{ServiceMemories, grants},
};
use ic_memory::{RuntimeMemory, ic_stable_structures::DefaultMemoryImpl};

pub(super) type Memory = RuntimeMemory<DefaultMemoryImpl>;
const AUTHORITY: &str = "blob-service";
ic_memory::ic_memory_range!(
    authority = AUTHORITY,
    start = 120,
    end = 136,
    mode = Allowed
);
ic_memory::eager_init!({
    for request in installation::requests(AUTHORITY).expect("service memory requests") {
        ic_memory::register_memory_request(request).expect("register service request");
    }
});

pub(super) struct Grants {
    pub(super) configuration: Memory,
    pub(super) stores: ServiceMemories<Memory>,
}
pub(super) fn open() -> Grants {
    let lookup = ic_memory::open_default_memory_manager_memory_by_key;
    Grants {
        configuration: lookup(INSTALLATION_MEMORY_KEY).expect("configuration grant"),
        stores: grants::open(lookup).expect("service grants"),
    }
}
