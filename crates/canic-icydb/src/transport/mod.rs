//! Bounded raw IC transport; native database codecs are supplied by the consumer.

/// Admit the raw argument length before allocating or copying ingress bytes.
/// Returns `None` on admission/allocation failure or outside Wasm execution.
#[must_use]
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        clippy::missing_const_for_fn,
        reason = "native refusal shares the runtime IC argument reader"
    )
)]
pub fn read_arguments(max_bytes: usize) -> Option<Vec<u8>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = max_bytes;
        None
    }
    #[cfg(target_arch = "wasm32")]
    {
        let length = ic0::msg_arg_data_size();
        if length > max_bytes {
            return None;
        }
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(length).ok()?;
        bytes.resize(length, 0);
        ic0::msg_arg_data_copy(&mut bytes, 0);
        Some(bytes)
    }
}

/// Serve one bounded raw call using the consumer's native byte contract.
///
/// Admission/decode refusal precedes the handler. Encoding failure traps so
/// synchronous handler mutations roll back; typed application failures are
/// ordinary encoded replies. Effects across earlier messages are not undone.
///
/// # Panics
/// Panics outside an IC entrypoint; traps on response encoding failure.
pub fn serve_raw<Q, R, E>(
    max_request_bytes: usize,
    decode: impl FnOnce(&[u8]) -> Result<Q, E>,
    encode: impl FnOnce(R) -> Result<Vec<u8>, E>,
    handler: impl FnOnce(Q) -> R,
) {
    let Some(bytes) = read_arguments(max_request_bytes) else {
        ic0::msg_reject(b"IcyDB request admission failed");
        return;
    };
    let Ok(request) = decode(&bytes) else {
        ic0::msg_reject(b"Invalid IcyDB request");
        return;
    };
    match encode(handler(request)) {
        Ok(response) => {
            ic0::msg_reply_data_append(&response);
            ic0::msg_reply();
        }
        Err(_) => ic0::trap(b"IcyDB response exceeds its contract"),
    }
}
