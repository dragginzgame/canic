/// Register one eager-init body for execution from the generated lifecycle hooks.
///
/// The body runs synchronously after Canic has restored memory, config, and
/// environment state, but before any zero-delay bootstrap timers fire.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        macro_rules! __canic_run_registered_eager_init {
            () => {{
                if option_env!("CANIC_SKIP_EAGER_INIT").is_none() {
                    $body
                }
            }};
        }
    };
}

/// Declare a thread-local static and schedule an eager initialization touch.
///
/// Expands to a `thread_local!` block and ensures the TLS slot is accessed
/// during the eager-init phase so subsequent calls observe a fully
/// initialized value.
#[macro_export]
macro_rules! eager_static {
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        const _: () = {
            fn __canic_touch_tls() {
                $name.with(|_| {});
            }

            #[ $crate::__reexports::ctor::ctor(anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_eager_tls() {
                $crate::runtime::defer_tls_initializer(__canic_touch_tls);
            }
        };
    };
}
