//! Explicit install and strict empty-argument reopen hooks.
#[doc(hidden)]
pub use ic_platform_runtime as runtime;

/// Mount one bounded install decoder and the application's install and reopen handlers.
/// The reopen handler takes no arguments. A custom upgrade payload can use Platform's
/// entrypoints directly. Panics in either handler trap the lifecycle message.
#[macro_export]
macro_rules! mount_lifecycle {
    (input = $input:ty, max_bytes = $max:literal, decode = $decode:path,
     install = $install:path, reopen = $reopen:path $(,)?) => {
        const _: () = {
            use $crate::lifecycle::runtime as platform;
            fn admit() -> Result<(), std::convert::Infallible> { Ok(()) }
            fn empty(bytes: &[u8]) -> Result<(), &'static str> {
                bytes.is_empty().then_some(()).ok_or("expected empty upgrade arguments")
            }
            fn fail(error: impl std::fmt::Debug) -> ! {
                platform::system::trap(&format!("lifecycle refused: {error:?}"))
            }
            #[platform::init(runtime = platform, guard = admit, decode = $decode, max_bytes = $max, on_error = fail)]
            #[allow(clippy::needless_pass_by_value)]
            fn init(input: $input) { $install(input); }
            #[platform::post_upgrade(runtime = platform, guard = admit, decode = empty, max_bytes = 0, on_error = fail)]
            fn post_upgrade((): ()) { $reopen(); }
        };
    };
}
