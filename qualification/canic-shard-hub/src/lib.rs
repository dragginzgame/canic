//! Installed hub witness. Its verified ingress identity supplies the private placement key.
mod actor;
mod routes;

canic::mount_hub_routes!();

mod paid_fixture;
