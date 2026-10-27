//! Native schema registration requires both Canic support and the consumer's
//! declared `wire-schema` feature, matching its conditional DTO derives.

/// Register one native projection owner; it may contribute several mounted methods.
#[doc(hidden)]
#[cfg(all(feature = "wire-schema", not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! __canic_register_wire_schema_provider {
    ($projection:expr) => {
        $crate::__canic_register_wire_schema_provider!($projection, None);
    };
    ($projection:expr, $adapter:expr) => {
        #[cfg(feature = "wire-schema")]
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

/// Declare a consumer-owned raw Wire entrypoint in both Wasm and native schemas.
/// The caller owns its export, codec and access enforcement; no handler is generated.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_register_raw_wire_schema {
    ($name:expr, $kind:ident, $id:expr, $request:ty, $response:ty, $input:expr, $output:expr) => {
        #[cfg(target_arch = "wasm32")]
        const _: () = {
            use $crate::__internal::contracts::protocol::wire_declaration::{
                CompiledMethod, HEADER_BYTES, MethodKind, MethodLimits,
            };
            #[used]
            #[unsafe(link_section = "icp:private canic:wire")]
            static DECLARATION: [u8; HEADER_BYTES + ($name).len()] = CompiledMethod {
                name: $name,
                kind: MethodKind::$kind,
                id: $id,
                request: MethodLimits {
                    bytes: ($input).bytes as u64,
                    allocation: ($input).allocation as u64,
                    nodes: ($input).nodes as u64,
                    depth: ($input).depth as u64,
                },
                response: MethodLimits {
                    bytes: ($output).bytes as u64,
                    allocation: ($output).allocation as u64,
                    nodes: ($output).nodes as u64,
                    depth: ($output).depth as u64,
                },
            }
            .encode();
        };
        $crate::__canic_register_wire_schema!(
            $name, $kind, $id, $request, $response, $input, $output
        );
    };
}
