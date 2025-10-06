///
/// Network
/// Identifies the environment the canister believes it runs in.
///
pub enum Network {
    Ic,
    Local,
}

///
/// get_network
/// Determine the current network from `DFX_NETWORK`.
///
#[must_use]
pub fn get_network() -> Option<Network> {
    match option_env!("DFX_NETWORK") {
        Some("local") => Some(Network::Local),
        Some("ic") => Some(Network::Ic),

        _ => None,
    }
}
