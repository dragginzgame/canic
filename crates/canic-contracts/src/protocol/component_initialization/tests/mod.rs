//! Native initialization graph includes the target principal and opaque byte vector.
#[cfg(feature = "wire-schema")]
#[test]
fn initialization_graph_is_complete() {
    ic_wire::typescript::native_schema::<
        crate::dto::component_initialization::RootComponentInitializationRequest,
    >()
    .unwrap();
    ic_wire::typescript::native_schema::<
        Result<crate::dto::role::OperationReceipt, crate::dto::error::Error>,
    >()
    .unwrap();
}
