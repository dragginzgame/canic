use crate::perf_scope;

/// Dispatch a synchronous query endpoint.
#[inline]
pub fn dispatch_query<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf_scope!("{}", label);

    // future hooks:
    // - app enabled guard
    // - readonly guard
    // - tracing
    // - metrics

    f()
}

/// Dispatch an async query endpoint.
#[inline]
pub async fn dispatch_query_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: std::future::Future<Output = R>,
{
    perf_scope!("{}", label);
    f().await
}

/// Dispatch a synchronous update endpoint.
#[inline]
pub fn dispatch_update<R>(label: &'static str, f: impl FnOnce() -> R) -> R {
    perf_scope!("{}", label);
    f()
}

/// Dispatch an async update endpoint.
#[inline]
pub async fn dispatch_update_async<R, F>(label: &'static str, f: impl FnOnce() -> F) -> R
where
    F: std::future::Future<Output = R>,
{
    perf_scope!("{}", label);
    f().await
}
