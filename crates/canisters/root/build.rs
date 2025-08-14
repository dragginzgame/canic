fn main() -> std::io::Result<()> {
    icu::icu_build!("../icu.toml");

    // Tell rustc that `icu_github_ci` is a valid cfg to avoid warnings.
    println!("cargo::rustc-check-cfg=cfg(github_ci)");

    // Check the GitHub Actions env var
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        // Emit a custom cfg flag to the compiler
        println!("cargo:rustc-cfg=github_ci");
    }

    Ok(())
}
