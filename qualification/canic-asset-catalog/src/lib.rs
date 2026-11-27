//! Embedded asset-catalog witness using the real Blob service and Canic adapter.
//! This qualification application has no provider transport or settlement logic.
mod canister;
pub mod protocol;

#[cfg(test)]
mod tests {
    #[test]
    fn blob_mounts_describe_both_wire_directions() {
        use canic::{blob, inspection};
        let methods = inspection::methods("canic_asset_catalog", &inspection::Schemas::default())
            .expect("mounted schemas from the Blob supplier");
        for (name, mode, id) in [
            ("canic_blob_update", "update", blob::UPDATE),
            ("canic_blob_query", "query", blob::QUERY),
            ("canic_blob_management", "update", blob::MANAGEMENT),
        ] {
            let method = &methods[name];
            assert_eq!((method.mode, method.id), (mode, id));
        }
    }
}
