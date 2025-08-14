fn main() -> std::io::Result<()> {
    icu::icu_build!("../icu.toml");

    Ok(())
}
