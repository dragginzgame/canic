fn main() {
    // Check the GitHub Actions env var
    if std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
        // Emit a custom cfg flag to the compiler
        println!("cargo:rustc-cfg=github_ci");
    }
}
