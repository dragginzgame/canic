use fixture_supported_domain::DOMAIN_VALUE;

#[unsafe(no_mangle)]
pub extern "C" fn canic_fixture_value() -> u8 {
    DOMAIN_VALUE
}

