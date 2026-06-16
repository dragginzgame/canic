use std::fmt::Write;

// Encode one string as a Candid text literal.
#[must_use]
pub(super) fn idl_text(value: &str) -> String {
    serde_json::to_string(value).expect("string literal encoding must succeed")
}

// Encode one blob as a Candid text blob literal.
#[must_use]
pub(super) fn idl_blob(bytes: &[u8]) -> String {
    let mut encoded = String::from("blob \"");

    for byte in bytes {
        let _ = write!(encoded, "\\{byte:02X}");
    }

    encoded.push('"');
    encoded
}
