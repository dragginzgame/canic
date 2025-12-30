use candid::Principal;

fn validate_principal(label: &str, text: &str) {
    if let Err(err) = Principal::from_text(text) {
        panic!("Invalid principal literal {label}: {text} ({err})");
    }
}

macro_rules! static_canisters {
    ($($name:ident = $id:expr;)+) => {{
        $(validate_principal(stringify!($name), $id);)+
    }};
}

macro_rules! sns_table {
    (
        $(
            $name:ident {
                root: $root:expr,
                governance: $gov:expr,
                index: $idx:expr,
                ledger: $led:expr $(,)?
            }
        ),+ $(,)?
    ) => {{
        $(
            validate_principal(concat!(stringify!($name), ".root"), $root);
            validate_principal(concat!(stringify!($name), ".governance"), $gov);
            validate_principal(concat!(stringify!($name), ".index"), $idx);
            validate_principal(concat!(stringify!($name), ".ledger"), $led);
        )+
    }};
}

fn main() {
    // Re-run this build script if the build-time network changes.
    // This ensures downstream code sees the correct value via env!/option_env!.
    println!("cargo:rerun-if-env-changed=DFX_NETWORK");
    println!("cargo:rerun-if-changed=src/env/ck.inc.rs");
    println!("cargo:rerun-if-changed=src/env/nns.inc.rs");
    println!("cargo:rerun-if-changed=src/env/sns.inc.rs");

    // Share the same principal data with build-time validation.
    include!("src/env/ck.inc.rs");
    include!("src/env/nns.inc.rs");
    include!("src/env/sns.inc.rs");
}
