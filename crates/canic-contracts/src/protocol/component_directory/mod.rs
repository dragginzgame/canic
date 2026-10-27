//! Typed member and controller directory reads under exact retained authority.

/// Registered component-member page query; never a controller bypass.
pub const MEMBER_PAGE: &str = "canic_component_directory_page";
/// Controller-authenticated exact directory head.
pub const HEAD: &str = "canic_root_directory_head";
/// Controller-authenticated exact directory page.
pub const PAGE: &str = "canic_root_directory_page";
/// Identity for registered-member reads.
pub const MEMBER_PAGE_ID: u64 = 0x4341_4e49_0000_0300;
/// Identity for controller head reads.
pub const HEAD_ID: u64 = 0x4341_4e49_0000_0301;
/// Identity for controller page reads.
pub const PAGE_ID: u64 = 0x4341_4e49_0000_0302;
/// Full authority, filters and opaque cursor; bounded before workflow entry.
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 65_536,
    allocation: 2_097_152,
    nodes: 65_536,
    depth: 64,
};
/// Bounded full authority pages, preserving the owning workflow's page-size ceiling.
pub const RESPONSE_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 2_097_152,
    allocation: 67_108_864,
    nodes: 4_194_304,
    depth: 64,
};
