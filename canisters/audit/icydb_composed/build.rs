fn main() {
    #[cfg(feature = "participant")]
    icydb::build::build_canister!(canic_composed_empty_schema::EmptyCanister)
        .expect("generate the selected IcyDB participant schema");
    canic::build!("canic.toml");
}
