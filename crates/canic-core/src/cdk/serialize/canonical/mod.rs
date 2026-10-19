//! Module: cdk::serialize::canonical
//!
//! Responsibility: encode protected configuration values with fixed-width canonical primitives.
//! Does not own: section field order, topology validation, hashing, or size policy.
//! Boundary: configuration compilers use unsigned big-endian values and length-prefixed bytes.

pub struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    pub fn new(domain: &[u8], schema_version: u32) -> Self {
        let mut encoder = Self { bytes: Vec::new() };
        encoder.bytes(domain);
        encoder.u32(schema_version);
        encoder
    }

    pub fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.bytes.extend_from_slice(value);
    }

    pub fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn u128(&mut self, value: u128) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_primitives_preserve_fixed_width_and_length_prefixes() {
        let mut encoder = CanonicalEncoder::new(b"abc", 1);
        encoder.u8(2);
        encoder.u32(0x0102_0304);
        encoder.u64(0x0807_0605_0403_0201);
        encoder.u128(u128::MAX);
        encoder.string("x");
        assert_eq!(encoder.finish(), b"\x00\x00\x00\x00\x00\x00\x00\x03abc\x00\x00\x00\x01\x02\x01\x02\x03\x04\x08\x07\x06\x05\x04\x03\x02\x01\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\xff\x00\x00\x00\x00\x00\x00\x00\x01x");
    }
}
