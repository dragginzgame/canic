//! Module: memory_macros
//!
//! Responsibility: adapt explicit Canic memory authorities to `ic-memory` registration.
//! Does not own: authority values, stable keys, memory IDs, or bootstrap ordering.
//! Boundary: registers declarations during static initialization and opens only committed slots.

// Register through the public function so Canic authority constants remain the
// single string definitions; the upstream macro intentionally accepts literals only.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_ic_memory_key {
    ($authority:expr, $stable_key:literal, $label:path, $id:expr) => {{
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_static_memory_declaration() {
                let _ = core::marker::PhantomData::<$label>;
                $crate::__reexports::ic_memory::register_static_memory_manager_declaration(
                    $id,
                    $authority,
                    stringify!($label),
                    $stable_key,
                )
                .expect("Canic static memory declaration failed");
            }
        };

        $crate::memory::runtime::assert_memory_bootstrap_ready(stringify!($label), $id);
        $crate::__reexports::ic_memory::open_default_memory_manager_memory($stable_key, $id)
            .expect("Canic failed to open committed stable memory; bootstrap must run first and the stable key/id must match the committed declaration")
    }};
}

/// Declare a stable-memory slot with an explicit authority and ABI-stable key.
///
/// Use this for every Canic-managed memory. The stable key, not crate or Rust
/// type identity, is the durable allocation identity.
#[macro_export]
macro_rules! ic_memory_key {
    (authority = CANIC_CORE_MEMORY_AUTHORITY, key = $stable_key:literal, ty = $label:path, id = $id:expr $(,)?) => {
        $crate::__canic_ic_memory_key!(
            $crate::memory::CANIC_CORE_MEMORY_AUTHORITY,
            $stable_key,
            $label,
            $id
        )
    };
    (authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, key = $stable_key:literal, ty = $label:path, id = $id:expr $(,)?) => {{
        $crate::__canic_ic_memory_key!(
            $crate::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            $stable_key,
            $label,
            $id
        )
    }};
    (authority = $authority:literal, key = $stable_key:literal, ty = $label:path, id = $id:expr $(,)?) => {{ $crate::__canic_ic_memory_key!($authority, $stable_key, $label, $id) }};
    (authority = $authority:path, key = $stable_key:literal, ty = $label:path, id = $id:expr $(,)?) => {{ $crate::__canic_ic_memory_key!($authority, $stable_key, $label, $id) }};
}

// Register an authority range from a centralized authority value.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_ic_memory_range {
    ($authority:expr, $start:expr, $end:expr, $mode:ident) => {
        const _: () = {
            #[ $crate::__reexports::ctor::ctor(unsafe, anonymous, crate_path = $crate::__reexports::ctor) ]
            fn __canic_register_static_memory_range() {
                $crate::__reexports::ic_memory::register_static_memory_manager_range(
                    $start,
                    $end,
                    $authority,
                    $crate::__reexports::ic_memory::MemoryManagerRangeMode::$mode,
                    None,
                )
                .expect("Canic static memory range declaration failed");
            }
        };
    };
}

/// Declare a MemoryManager ID range owned by an explicit authority.
#[macro_export]
macro_rules! ic_memory_range {
    (authority = CANIC_CORE_MEMORY_AUTHORITY, start = $start:expr, end = $end:expr $(,)?) => {
        $crate::__canic_ic_memory_range!(
            $crate::memory::CANIC_CORE_MEMORY_AUTHORITY,
            $start,
            $end,
            Reserved
        );
    };
    (authority = CANIC_CORE_MEMORY_AUTHORITY, start = $start:expr, end = $end:expr, mode = $mode:ident $(,)?) => {
        $crate::__canic_ic_memory_range!(
            $crate::memory::CANIC_CORE_MEMORY_AUTHORITY,
            $start,
            $end,
            $mode
        );
    };
    (authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, start = $start:expr, end = $end:expr $(,)?) => {
        $crate::__canic_ic_memory_range!(
            $crate::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            $start,
            $end,
            Reserved
        );
    };
    (authority = CANIC_CONTROL_PLANE_MEMORY_AUTHORITY, start = $start:expr, end = $end:expr, mode = $mode:ident $(,)?) => {
        $crate::__canic_ic_memory_range!(
            $crate::memory::CANIC_CONTROL_PLANE_MEMORY_AUTHORITY,
            $start,
            $end,
            $mode
        );
    };
    (authority = $authority:literal, start = $start:expr, end = $end:expr $(,)?) => {
        $crate::__canic_ic_memory_range!($authority, $start, $end, Reserved);
    };
    (authority = $authority:literal, start = $start:expr, end = $end:expr, mode = $mode:ident $(,)?) => {
        $crate::__canic_ic_memory_range!($authority, $start, $end, $mode);
    };
    (authority = $authority:path, start = $start:expr, end = $end:expr $(,)?) => {
        $crate::__canic_ic_memory_range!($authority, $start, $end, Reserved);
    };
    (authority = $authority:path, start = $start:expr, end = $end:expr, mode = $mode:ident $(,)?) => {
        $crate::__canic_ic_memory_range!($authority, $start, $end, $mode);
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
