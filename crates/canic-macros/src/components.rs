//! Check the requested child against the selected hub before emitting its fleet requirement.
use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::LitStr;

pub(super) fn expand(kind: &str, input: TokenStream) -> syn::Result<TokenStream> {
    let pool: LitStr = syn::parse2(input)?;
    let fail = |message: String| syn::Error::new(pool.span(), message);
    let package = std::env::var("CARGO_PKG_NAME").map_err(|error| fail(error.to_string()))?;
    let selection = format!("CANIC_APP_CHILDREN_{package}");
    let tracked_config = if let Ok(children) = std::env::var(&selection) {
        if !children
            .split_ascii_whitespace()
            .any(|child| child == pool.value())
        {
            return Err(fail(format!(
                "{:?} is not a direct child of the selected hub for package {package:?}",
                pool.value()
            )));
        }
        TokenStream::new()
    } else {
        // Native Cargo and rust-analyzer do not run the Canic planner.
        let manifest = PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR")
                .ok_or_else(|| fail("missing Cargo manifest directory".into()))?,
        );
        let path = manifest.ancestors().map(|dir| dir.join("canic.toml")).find(|file| file.is_file())
            .ok_or_else(|| fail(format!("no canic.toml above package {package:?}; build with canic app build for an external config")))?;
        let source = std::fs::read_to_string(&path).map_err(|error| fail(error.to_string()))?;
        let config: toml::Table =
            toml::from_str(&source).map_err(|error| fail(error.to_string()))?;
        check_child(&config, &package, &pool.value()).map_err(fail)?;
        let file = path
            .to_str()
            .ok_or_else(|| fail("config path must be UTF-8".into()))?;
        quote!(
            const _: &[u8] = include_bytes!(#file);
        )
    };
    let section = format!("canic:pool:{kind}:{}", pool.value());
    Ok(quote! {{
        const _: Option<&str> = option_env!(#selection);
        #tracked_config
        ::canic::__canic_report_mount!(#section);
        #pool
    }})
}

pub(super) fn check_child(config: &toml::Table, package: &str, child: &str) -> Result<(), String> {
    let apps = config
        .get("apps")
        .and_then(toml::Value::as_table)
        .ok_or("missing apps in canic.toml")?;
    let mut owners = apps
        .values()
        .filter_map(|app| app.get("canisters")?.as_table())
        .flat_map(|components| {
            components
                .iter()
                .filter(move |(_, component)| {
                    component
                        .get("build")
                        .and_then(|build| build.get("package"))
                        .and_then(toml::Value::as_str)
                        == Some(package)
                })
                .map(move |(name, _)| (name, components))
        });
    let (hub, components) = owners
        .next()
        .ok_or_else(|| format!("canic.toml has no hub build recipe for package {package:?}"))?;
    if owners.next().is_some() {
        return Err(format!(
            "multiple components build package {package:?}; select the app and canister with canic app build"
        ));
    }
    if components
        .get(child)
        .and_then(|component| component.get("parent"))
        .and_then(toml::Value::as_str)
        != Some(hub)
    {
        return Err(format!(
            "{child:?} is not a direct child of hub {hub:?} in canic.toml"
        ));
    }
    Ok(())
}
