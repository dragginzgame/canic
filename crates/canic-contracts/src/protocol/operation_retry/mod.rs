//! Bounded controller methods for retained business-attempt windows.
#[cfg(test)]
mod tests;
pub const REQUEST_LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 65_536,
    nodes: 256,
    depth: 16,
};
pub const RESPONSE_LIMITS: ic_wire::Limits = REQUEST_LIMITS;
/// Observe the exact durable retry window.
pub mod status {
    pub const METHOD: &str = "canic_operation_retry_status";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3c00;
}
/// Explicitly renew the observed suspended window without changing effect authority.
pub mod resume {
    pub const METHOD: &str = "canic_operation_retry_resume";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_3c01;
}
