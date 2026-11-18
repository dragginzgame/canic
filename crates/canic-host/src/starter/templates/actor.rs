//! Select adapters explicitly; mounting performs no initialization or activation.
canic::mount_greeting_endpoints!(crate::greeting);
canic::mount_readiness_endpoint!(crate::actor::readiness);

fn readiness() -> canic::readiness::State {
    // This stateless application has no storage or services to open.
    canic::readiness::State::Ready
}
