///
/// Network
///

pub enum Network {
    Ic,
    Local,
}

// get_network
#[must_use]
pub fn get_network() -> Option<Network> {
    match option_env!("DFX_NETWORK") {
        Some("local") => Some(Network::Local),
        Some("ic") => Some(Network::Ic),

        _ => None,
    }
}
