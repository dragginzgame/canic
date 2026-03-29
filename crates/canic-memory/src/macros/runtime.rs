/// Register one eager-init body for execution during lifecycle bootstrap.
///
/// The body is registered during process startup and later runs synchronously
/// after Canic has touched eager TLS and before the memory registry flush.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        const _: () = {
            fn __canic_registered_eager_init_body() {
                $body
            }

            #[ $crate::__reexports::ctor::ctor(anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_eager_init() {
                $crate::runtime::defer_eager_init(__canic_registered_eager_init_body);
            }
        };
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
