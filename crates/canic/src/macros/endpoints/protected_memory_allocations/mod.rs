//! Controller-only memory allocations with role-owned activation fencing.
#[doc(hidden)]
#[macro_export]
macro_rules! __canic_emit_protected_memory_allocations_endpoint {
    (before = [$($before:stmt)*]) => {
        #[$crate::query(requires(caller::is_controller()), wire(id = ::canic::protocol::protected_memory_allocations::METHOD_ID, request = ::canic::protocol::protected_memory_allocations::REQUEST_LIMITS, response = ::canic::protocol::protected_memory_allocations::RESPONSE_LIMITS))]
        fn canic_memory_allocations() -> Result<::canic::dto::memory::MemoryAllocationsResponse, ::canic::Error> {
            $($before)*
            $crate::__internal::core::api::memory::MemoryQuery::allocations()
        }
    };
}
