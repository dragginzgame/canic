//! Explicit install and strict empty-argument reopen hooks.
#[doc(hidden)]
pub use ic_platform_runtime as runtime;

#[doc(hidden)]
pub fn empty_arguments(bytes: &[u8]) -> Result<(), &'static str> {
    bytes
        .is_empty()
        .then_some(())
        .ok_or("expected empty lifecycle arguments")
}

/// Mount zero-argument install and reopen handlers, or supply a bounded install decoder and a typed install handler.
/// The reopen handler takes no arguments. A custom upgrade payload can use Platform's
/// entrypoints directly. Panics in either handler trap the lifecycle message.
// Rustfmt 1.100 repeatedly reindents multiline attributes in this macro body.
#[rustfmt::skip]
#[macro_export]
macro_rules! mount_lifecycle {
    (input = $input:ty, max_bytes = $max:literal, decode = $decode:path,
     install = $install:path, reopen = $reopen:path $(,)?) => {
        $crate::mount_lifecycle!(@hooks input: $input, $max, $decode, $install(input), $reopen);
    };
    (install = $install:path, reopen = $reopen:path $(,)?) => {
        $crate::mount_lifecycle!(@hooks (): (), 0, $crate::lifecycle::empty_arguments, $install(), $reopen);
    };
    (@hooks $binding:tt: $input:ty, $max:literal, $decode:path,
     $call:expr, $reopen:path) => {
        const _: () = {
            use $crate::lifecycle::runtime as platform;

            fn admit() -> Result<(), std::convert::Infallible> {
                Ok(())
            }

            fn fail(error: impl std::fmt::Debug) -> ! {
                platform::system::trap(&format!("lifecycle refused: {error:?}"))
            }

            #[platform::init(
                runtime = platform,
                guard = admit,
                decode = $decode,
                max_bytes = $max,
                on_error = fail,
            )]
            #[allow(clippy::needless_pass_by_value)]
            fn init($binding: $input) {
                $call;
            }

            #[platform::post_upgrade(
                runtime = platform,
                guard = admit,
                decode = $crate::lifecycle::empty_arguments,
                max_bytes = 0,
                on_error = fail,
            )]
            fn post_upgrade((): ()) {
                $reopen();
            }
        };
    };
}
