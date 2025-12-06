/// Run `$body` during process start-up using `ctor`.
///
/// The macro expands to a `ctor` hook so eager TLS initializers can register
/// their work before any canister lifecycle hooks execute. Prefer wrapping
/// the body in a separate function for larger initializers to keep the hook
/// simple.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        #[ $crate::export::ctor::ctor(anonymous, crate_path = $crate::export::ctor) ]
        fn __canic_eager_init() {
            $body
        }
    };
}

/// Declare a thread-local static and schedule an eager initialization touch.
///
/// Expands to a `thread_local!` block and ensures the TLS slot is accessed
/// during the eager-init phase so subsequent calls observe a fully
/// initialized value. Use this for caches that must exist before canister
/// entry points run.
#[macro_export]
macro_rules! eager_static {
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        // A simple wrapper function that forces TLS initialization.
        fn __touch_tls() {
            $name.with(|_| {});
        }

        $crate::eager_init!({
            $crate::runtime::defer_tls_initializer(__touch_tls);
        });
    };
}
