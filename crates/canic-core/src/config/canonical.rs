//! Module: config::canonical
//!
//! Responsibility: encode protected configuration values with fixed-width canonical primitives.
//! Does not own: section field order, topology validation, hashing, or size policy.
//! Boundary: configuration compilers use unsigned big-endian values and length-prefixed bytes.

pub(super) struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    pub(super) fn new(domain: &[u8], schema_version: u32) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(schema_version);
        encoder
    }

    pub(super) fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    pub(super) fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(super) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(super) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(super) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
