//! Bounded platform IO; payload codecs and reply contracts are owned by Blob.

#[doc(hidden)]
#[must_use]
pub fn arguments(maximum: usize) -> Vec<u8> {
    let length = ic_platform::ic0::msg_arg_data_size();
    assert!(length <= maximum, "Blob ingress exceeds its byte bound");
    let mut bytes = vec![0; length];
    ic_platform::ic0::msg_arg_data_copy(&mut bytes, 0);
    bytes
}
