// -----------------------------------------------------------------------------
// Auth macros
// -----------------------------------------------------------------------------

/// Enforce that every supplied rule future succeeds for the current caller.
///
/// This is a convenience wrapper around `require_all`, allowing guard
/// checks to stay in expression position within async endpoints.
#[macro_export]
macro_rules! auth_require_all {
    ($($f:expr),* $(,)?) => {{
        $crate::__internal::core::access::auth::require_all(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}

/// Enforce that at least one supplied rule future succeeds for the current
/// caller.
///
/// See [`auth_require_all!`] for details on accepted rule shapes.
#[macro_export]
macro_rules! auth_require_any {
    ($($f:expr),* $(,)?) => {{
        $crate::__internal::core::access::auth::require_any(vec![
            $( Box::new(move |caller| Box::pin($f(caller))) ),*
        ]).await
    }};
}
