//! Internal-fixture-only reply scheduling controls; no production barrier is emitted.
pub const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 1024,
    allocation: 4096,
    nodes: 32,
    depth: 8,
};
pub mod arm {
    pub const METHOD: &str = "canic_test_fixture_reply_arm";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1b00;
}
pub mod release {
    pub const METHOD: &str = "canic_test_fixture_reply_release";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1b01;
}
pub mod waiting {
    pub const METHOD: &str = "canic_test_fixture_reply_waiting";
    pub const METHOD_ID: u64 = 0x4341_4e49_0000_1b02;
}

#[cfg(test)]
mod tests;
