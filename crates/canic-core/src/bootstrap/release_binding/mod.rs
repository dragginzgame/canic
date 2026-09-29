//! Fixed release identity storage read by generated lifecycle entrypoints.
//!
//! Host finalization fills an unbound slot before optimization and qualification.
//! Runtime reads its own installed bytes; install arguments cannot supply this value.

/// Prefix identifying the maintained release binding data layout.
pub const RELEASE_BINDING_PREFIX: &[u8] = b"CANIC_RELEASE_BUILD_SLOT_V1[";
/// Suffix delimiting the release binding independently of adjacent linker data.
pub const RELEASE_BINDING_SUFFIX: &[u8] = b"]CANIC_RELEASE_BUILD_SLOT_END";
/// Canonical hexadecimal identity width.
pub const RELEASE_BINDING_ID_BYTES: usize = 64;
/// Complete slot width, unchanged by host binding.
pub const RELEASE_BINDING_BYTES: usize =
    RELEASE_BINDING_PREFIX.len() + RELEASE_BINDING_ID_BYTES + RELEASE_BINDING_SUFFIX.len();

/// Linker-retained storage for a release identity supplied before installation.
#[repr(transparent)]
pub struct EmbeddedReleaseBuildId([u8; RELEASE_BINDING_BYTES]);

impl EmbeddedReleaseBuildId {
    /// Construct an unbound template or a directly compiled fixture identity.
    ///
    /// # Panics
    /// Panics if a supplied identity is not canonical lowercase hexadecimal.
    #[must_use]
    pub const fn new(identity: Option<&str>) -> Self {
        let mut bytes = [b'?'; RELEASE_BINDING_BYTES];
        let mut index = 0;
        while index < RELEASE_BINDING_PREFIX.len() {
            bytes[index] = RELEASE_BINDING_PREFIX[index];
            index += 1;
        }
        if let Some(identity) = identity {
            let identity = identity.as_bytes();
            assert!(identity.len() == RELEASE_BINDING_ID_BYTES);
            let mut offset = 0;
            while offset < RELEASE_BINDING_ID_BYTES {
                let byte = identity[offset];
                assert!(byte.is_ascii_digit() || (byte >= b'a' && byte <= b'f'));
                bytes[index + offset] = byte;
                offset += 1;
            }
        }
        index += RELEASE_BINDING_ID_BYTES;
        let mut offset = 0;
        while offset < RELEASE_BINDING_SUFFIX.len() {
            bytes[index + offset] = RELEASE_BINDING_SUFFIX[offset];
            offset += 1;
        }
        Self(bytes)
    }

    /// Read the installed data rather than a compile-time copy of the template.
    ///
    /// # Panics
    /// Panics if the installed slot is malformed.
    #[must_use]
    pub fn read(&self) -> Option<String> {
        // SAFETY: each pointer addresses a live, initialized, aligned byte.
        // Host binding changes the Wasm file before loading;
        // runtime never writes this allocation. Volatile prevents LLVM/LTO from
        // substituting the unbound compile-time bytes for the installed value.
        let bytes = self
            .0
            .each_ref()
            .map(|byte| unsafe { std::ptr::read_volatile(byte) });
        let start = RELEASE_BINDING_PREFIX.len();
        let end = start + RELEASE_BINDING_ID_BYTES;
        assert_eq!(&bytes[..start], RELEASE_BINDING_PREFIX);
        assert_eq!(&bytes[end..], RELEASE_BINDING_SUFFIX);
        let identity = &bytes[start..end];
        if identity.iter().all(|byte| *byte == b'?') {
            return None;
        }
        assert!(identity.iter().all(u8::is_ascii_hexdigit));
        assert!(!identity.iter().any(u8::is_ascii_uppercase));
        Some(String::from_utf8(identity.to_vec()).expect("canonical release identity"))
    }
}
