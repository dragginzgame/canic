fn main() {
    canic::build!("canic.toml");

    println!("cargo:rerun-if-env-changed=CANIC_GENERIC_COHORT_WIDTH");
    for width in 2..=5 {
        println!("cargo:rustc-check-cfg=cfg(canic_generic_cohort_ge_{width})");
    }

    let width = std::env::var("CANIC_GENERIC_COHORT_WIDTH")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u8>()
        .expect("CANIC_GENERIC_COHORT_WIDTH must be an integer from 1 through 5");
    assert!(
        (1..=5).contains(&width),
        "CANIC_GENERIC_COHORT_WIDTH must be from 1 through 5"
    );
    for threshold in 2..=width {
        println!("cargo:rustc-cfg=canic_generic_cohort_ge_{threshold}");
    }
}
