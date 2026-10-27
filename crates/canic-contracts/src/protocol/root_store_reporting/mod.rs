//! Root-owned Store observations; fixture progress remains a composite query.
pub const REQUEST_LIMITS: ic_wire::Limits = crate::protocol::store_inventory::REQUEST_LIMITS;
pub const RESPONSE_LIMITS: ic_wire::Limits = crate::protocol::store_inventory::RESPONSE_LIMITS;
pub mod overview {
    pub const METHOD: &str = "canic_root_store_overview";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1c00;
}
pub mod fixture {
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1c01;
}
#[cfg(test)]
mod tests;
