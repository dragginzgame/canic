//! Raw application Wire exports share the adapter's IC entrypoint owner.

/// Export one synchronous application IC Wire method with its contract identity.
///
/// The native contract owner supplies bounded decoding/encoding in the body.
/// This adapter owns the IC export, executor context and artifact declaration.
/// `mode` is `query` or `update`; the body must explicitly reply or reject.
#[macro_export]
macro_rules! wire_endpoint {
    ($(#[$attribute:meta])* $mode:ident, $name:literal, $contract:expr, $body:block) => {
        $(#[$attribute])*
        const _: () = {
            use $crate::__private::cdk as __canic_wire_sdk;

            #[$crate::__private::native_entry($mode, name = $name, sdk = __canic_wire_sdk)]
            fn __canic_wire_export() {
                #[used]
                #[expect(unsafe_code, reason = "the host export carries its native contract identity")]
                #[unsafe(link_section = "icydb:wire")]
                static CONTRACT: [u8; 8] = ($contract).to_le_bytes();
                $body
            }
        };
    };
}
