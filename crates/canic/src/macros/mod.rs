//! Public macro entry points used across Canic.
//!
//! The submodules host build-time helpers (`build`), canister lifecycle
//! scaffolding (`start`), memory registry shortcuts (`memory`), thread-local
//! eager initializers (`thread`), and endpoint generators (`endpoints`). This
//! top-level module exposes lightweight instrumentation macros shared across
//! those layers.

pub mod build;
pub mod endpoints;
pub mod memory;
pub mod runtime;
pub mod start;

/// Run `$body` during process start-up using `ctor`.
///
/// The macro expands to a `ctor` hook so eager TLS initializers can register
/// their work before any canister lifecycle hooks execute. Prefer wrapping
/// the body in a separate function for larger initializers to keep the hook
/// simple.
#[macro_export]
macro_rules! eager_init {
    ($body:block) => {
        #[ $crate::export::ctor::ctor(anonymous, crate_path = $crate::export::ctor) ]
        fn __canic_eager_init() {
            $body
        }
    };
}
