//! Compile the storage composition used by a receiver hub without an application database.
canic::mount_receiver_lifecycle! {
    hub,
    config_key = "example.config.v1",
    authority = "example",
    reserved_keys = ["example.issuer.v1"],
    auth_read = "read",
}
