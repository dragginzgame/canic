#[unsafe(no_mangle)]
pub extern "C" fn canic_fixture_value() -> usize {
    fixture_protected_helper::protected_value()
}
