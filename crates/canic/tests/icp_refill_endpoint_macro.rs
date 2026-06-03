#![cfg(feature = "icp-refill")]

mod fixture {
    canic::canic_emit_icp_refill_endpoints!(guard = caller::is_controller());

    #[test]
    fn icp_refill_endpoint_macro_accepts_host_guard() {
        let _ = canic_icp_refill;
    }
}
