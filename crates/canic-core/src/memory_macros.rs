/// Declare a stable-memory slot with an explicit ABI-stable key.
///
/// Use this for every Canic-managed memory. The stable key, not crate or Rust
/// type identity, is the durable allocation identity.
#[macro_export]
macro_rules! ic_memory_key {
    ($stable_key:literal, $label:path, $id:expr) => {{
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_declare_memory_slot() {
                $crate::memory::registry::declare_memory_slot_with_key(
                    $id,
                    env!("CARGO_PKG_NAME"),
                    stringify!($label),
                    $stable_key,
                )
                .expect("memory id declaration validation failed");
            }
        };

        let _type_check: Option<$label> = None;

        $crate::memory::open_validated_memory($stable_key, stringify!($label), $id)
    }};
}

/// Register one eager-init body for execution during lifecycle bootstrap.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        const _: () = {
            fn __canic_registered_eager_init_body() {
                $body
            }

            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_eager_init() {
                $crate::memory::runtime::defer_eager_init(__canic_registered_eager_init_body);
            }
        };
    };
}

/// Declare a thread-local static and schedule an eager initialization touch.
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

            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_eager_tls() {
                $crate::memory::runtime::defer_tls_initializer(__canic_touch_tls);
            }
        };
    };
}
