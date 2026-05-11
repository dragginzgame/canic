pub fn default_icp() -> String {
    "icp".to_string()
}

pub fn local_network() -> String {
    "local".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Keep omitted --network behavior tied to the local replica.
    #[test]
    fn local_network_is_always_local() {
        assert_eq!(local_network(), "local");
    }
}
