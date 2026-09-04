use super::*;

#[test]
fn root_home_installation_warning_names_the_actual_directory() {
    let warning = root_home_warning_for(Some(OsStr::new("/")), Path::new("/.local/bin"))
        .expect("root HOME must warn");

    assert!(warning.contains("HOME resolves to `/`"));
    assert!(warning.contains("/.local/bin"));
    assert!(root_home_warning_for(Some(OsStr::new("/home/operator")), Path::new("/bin")).is_none());
}
