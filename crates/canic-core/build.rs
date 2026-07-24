fn main() {
    println!("cargo:rustc-check-cfg=cfg(canic_is_root)");
    println!("cargo:rerun-if-env-changed=CANIC_RELEASE_BUILD_ID");

    let network = std::env::var("ICP_ENVIRONMENT").unwrap_or_else(|_| {
        // Explicit, intentional default for local development.
        "local".to_string()
    });

    match network.as_str() {
        "local" | "ic" => {
            println!("cargo:rustc-env=ICP_ENVIRONMENT={network}");
        }
        other => {
            panic!(
                "ICP_ENVIRONMENT is invalid.\n\
Expected: local or ic\n\
Got: '{other}'\n\
Hint: unset ICP_ENVIRONMENT to default to 'local'."
            );
        }
    }

    if let Ok(release_build_id) = std::env::var("CANIC_RELEASE_BUILD_ID") {
        if release_build_id.len() != 64
            || !release_build_id
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            panic!(
                "CANIC_RELEASE_BUILD_ID is invalid.\n\
Expected: exactly 64 lowercase hexadecimal characters\n\
Got: '{release_build_id}'"
            );
        }
        println!("cargo:rustc-env=CANIC_RELEASE_BUILD_ID={release_build_id}");
    }
}
