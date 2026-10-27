//! Protected import authority and progress reads, independent of paid command execution.
pub const REQUEST_LIMITS: ic_wire::Limits = crate::protocol::pool_census::REQUEST_LIMITS;
pub const RESPONSE_LIMITS: ic_wire::Limits = crate::protocol::pool_census::RESPONSE_LIMITS;
pub mod context {
    pub const METHOD: &str = "canic_root_pool_import_context";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1f00;
}
pub mod status {
    pub const METHOD: &str = "canic_root_pool_import_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1f01;
}

#[cfg(test)]
mod tests;
