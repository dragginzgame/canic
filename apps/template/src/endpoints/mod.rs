canic::mount_greeting_endpoints!(crate::endpoints::greeting);

canic::mount_readiness_endpoint!(crate::endpoints::readiness);

fn greeting(name: &str) -> canic::Greeting {
    canic::Greeting {
        message: format!("Hello from Canic, {name}!"),
    }
}

fn readiness() -> canic::readiness::State {
    canic::readiness::State::Ready
}
