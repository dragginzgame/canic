//! Feature selection belongs to Canic, not an invented consuming-crate cfg.

/// Register one native projection owner; it may contribute several mounted methods.
#[doc(hidden)]
#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! __canic_register_wire_schema_provider {
    ($projection:expr) => {
        $crate::__canic_register_wire_schema_provider!($projection, None);
    };
    ($projection:expr, $adapter:expr) => {
        const _: () = {
            #[$crate::__internal::core::__reexports::ctor::ctor(
                        unsafe, anonymous, crate_path = $crate::__internal::core::__reexports::ctor
                    )]
            fn register() {
                $crate::endpoint::schema::register($projection, $adapter);
            }
        };
    };
}

/// Runtime builds acquire no metadata or registration owner.
#[doc(hidden)]
#[cfg(not(all(feature = "wire-schema", not(target_arch = "wasm32"))))]
#[macro_export]
macro_rules! __canic_register_wire_schema_provider {
    ($($projection:tt)*) => {};
}

/// Project a typed declaration through the same native registration owner.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_register_wire_schema {
    ($name:expr, $kind:ident, $id:expr, $request:ty, $response:ty, $input:expr, $output:expr) => {
        $crate::__canic_register_wire_schema_provider!(|| {
            Ok(vec![$crate::endpoint::schema::Method::describe::<
                $request,
                $response,
            >(
                $name,
                $crate::endpoint::schema::MethodKind::$kind,
                $id,
                $input,
                $output,
            )?])
        });
    };
}
