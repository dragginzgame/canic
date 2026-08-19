fn main() -> Result<(), Box<dyn std::error::Error>> {
    icydb::build::build_canister!(canic_icydb_lifecycle_schema::CanicIcydbLifecycleCanister)?;
    canic::build!("canic.toml");
    Ok(())
}
