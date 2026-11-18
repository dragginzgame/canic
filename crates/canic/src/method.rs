//! Named application Wire endpoints with Platform execution.
#[doc(hidden)]
pub use {ic_platform_runtime as runtime, ic_wire as wire, paste::paste};

/// Default Wire budgets for small application requests and replies.
pub const LIMITS: wire::Limits = wire::Limits {
    bytes: 512,
    allocation: 16384,
    nodes: 128,
    depth: 8,
};

/// Export the Rust function name and encode its signature with Wire. An empty
/// parameter list takes a unit frame. By default the method ID is Wire's hash
/// of the export name, both directions use [`method::LIMITS`](crate::method::LIMITS),
/// and there is no caller restriction. The handler owns application permissions.
///
/// Options, in order, are `name`, `method`, `limits` (both directions),
/// `reply_limits`, and `guard` (runs before argument copy). Use `async update`
/// for handlers that await; recheck affected state and authority after suspension.
///
/// ```ignore
/// canic::mount_wire_method! {
///     query(method = signup::GET_ME) fn get_me() -> Option<MyProfile> {
///         ops::user::profile(&auth::identity(api::caller(), "read"))
///     }
/// }
/// ```
#[macro_export]
macro_rules! mount_wire_method {
    (query $($spec:tt)*) => { $crate::mount_wire_method!(@parse query [] [] $($spec)*); };
    (update $($spec:tt)*) => { $crate::mount_wire_method!(@parse update [] [] $($spec)*); };
    (async update $($spec:tt)*) => { $crate::mount_wire_method!(@parse update [async] [.await] $($spec)*); };
    (@parse $kind:ident $async:tt $await:tt $(($($opts:tt)*))?
        fn $handler:ident ($($arg:ident: $request:ty)?) -> $reply:ty $body:block) => {
        $crate::mount_wire_method!(@options $handler [$kind $async $await fn $handler ($($arg: $request)?) -> $reply $body]
            $($($opts)*,)?);
    };
    (@options $handler:ident $item:tt name = $export:literal, $($rest:tt)*) => {
        $crate::mount_wire_method!(@emit $item [$export] [$export] $($rest)*);
    };
    (@options $handler:ident $item:tt $($rest:tt)*) => {
        $crate::mount_wire_method!(@emit $item ["" $handler] [match stringify!($handler).as_bytes() { [b'r', b'#', ..] => stringify!($handler).split_at(2).1, _ => stringify!($handler) }] $($rest)*);
    };
    (@emit [$kind:ident [$($async:tt)*] [$($await:tt)*] fn $handler:ident
        ($($arg:ident: $request:ty)?) -> $reply:ty $body:block]
        [$($export:tt)+] [$name:expr] $(method = $id:expr,)? $(limits = $limits:expr,)?
        $(reply_limits = $output:expr,)? $(guard = $guard:path,)? $(,)?) => {
        #[allow(clippy::needless_pass_by_value)]
        $($async)* fn $handler($($arg: $request)?) -> $reply $body
        const _: () = {
            use $crate::method::{runtime as platform, wire};
            type __CanicInput = $crate::mount_wire_method!(@default [()] $($request)?);
            const __CANIC_INPUT: wire::Limits = $crate::mount_wire_method!(@default [$crate::method::LIMITS] $($limits)?);
            const __CANIC_OUTPUT: wire::Limits = $crate::mount_wire_method!(@default [__CANIC_INPUT] $($output)?);
            $crate::method::paste! {
                const __CANIC_ID: u64 = $crate::mount_wire_method!(@default [wire::method_id($name)] $($id)?);
                fn __canic_admit() -> Result<(), String> {
                    $($guard().map_err(|error| format!("{error:?}"))?;)?
                    if platform::system::argument_size() > __CANIC_INPUT.bytes {
                        return Err("request byte limit exceeded".into());
                    }
                    Ok(())
                }
                fn __canic_decode(bytes: &[u8]) -> Result<__CanicInput, wire::Error> {
                    wire::decode(bytes, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Request }, __CANIC_INPUT)
                }
                #[allow(clippy::needless_pass_by_value)]
                fn __canic_encode(value: $reply) -> Result<Vec<u8>, wire::Error> {
                    wire::encode(&value, wire::Encoding::Frame { method: __CANIC_ID, direction: wire::Direction::Response }, __CANIC_OUTPUT)
                }
                // Platform requires literal export names and copy ceilings. Admission
                // enforces the selected Wire budget before Platform reads arguments.
                use $crate::method::fail as __canic_fail;
                #[platform::$kind(runtime = platform, guard = __canic_admit, decode = __canic_decode, max_bytes = 4294967295,
                    on_error = __canic_fail, name = $($export)+, encode = __canic_encode)]
                $($async)* fn __canic_invoke(input: __CanicInput) -> $reply {
                    $crate::mount_wire_method!(@call $handler input $($arg)?)$($await)*
                }
            }
        };
    };
    (@default [$($default:tt)*] $value:tt) => { $value };
    (@default [$($default:tt)*]) => { $($default)* };
    (@call $handler:ident $input:ident $arg:ident) => { $handler($input) };
    (@call $handler:ident $input:ident) => {{ let () = $input; $handler() }};
}

/// Trap a refused execution stage in its current message.
/// # Panics
/// Always traps.
#[doc(hidden)]
pub fn fail(error: impl std::fmt::Debug) -> ! {
    runtime::system::trap(&format!("endpoint refused: {error:?}"))
}
