/// Declare a stable-memory slot with an explicit ABI-stable key.
///
/// Use this for every Canic-managed memory. The stable key, not crate or Rust
/// type identity, is the durable allocation identity.
#[macro_export]
macro_rules! ic_memory_key {
    ($stable_key:literal, $label:path, $id:expr) => {{
        $crate::memory::runtime::assert_memory_bootstrap_ready(stringify!($label), $id);
        $crate::__reexports::ic_memory::ic_memory_key!($stable_key, $label, $id)
    }};
}

/// Declare a MemoryManager ID range owned by the declaring crate.
#[macro_export]
macro_rules! ic_memory_range {
    (start = $start:expr, end = $end:expr $(,)?) => {
        $crate::__reexports::ic_memory::ic_memory_range!(start = $start, end = $end,);
    };
    (start = $start:expr, end = $end:expr, mode = $mode:ident $(,)?) => {
        $crate::__reexports::ic_memory::ic_memory_range!(start = $start, end = $end, mode = $mode,);
    };
}

/// Register one eager-init body for execution during lifecycle bootstrap.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        $crate::__reexports::ic_memory::eager_init!($body);
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
