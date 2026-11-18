//! Bind the conventional ULID layout to the existing lifecycle and row adapters.
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Expr, Ident, LitInt, LitStr, Path, Token,
    parse::{ParseStream, Parser},
    spanned::Spanned,
};

pub(super) fn expand(input: TokenStream) -> syn::Result<TokenStream> {
    parse.parse2(input)
}

fn parse(input: ParseStream<'_>) -> syn::Result<TokenStream> {
    let role: Expr = input.parse()?;
    input.parse::<Token![,]>()?;
    let row: Path = input.parse()?;
    input.parse::<Token![=>]>()?;
    let resource: Path = input.parse()?;
    input.parse::<Token![,]>()?;

    let mut role_name = None::<Expr>;
    let mut database = None::<LitStr>;
    let mut authority = None::<LitStr>;
    let mut config = None::<LitStr>;
    let mut prefix = None::<LitStr>;
    let mut parent_storage_key = None::<LitStr>;
    let mut pages = None::<LitInt>;
    let mut payload = None::<Ident>;
    let mut status = None::<Path>;
    let mut generator = None::<Path>;
    let mut endpoint = None::<Ident>;
    let mut read_scope = None::<LitStr>;
    let mut write_scope = None::<LitStr>;
    syn::meta::parser(|option| {
        macro_rules! field {
            ($slot:ident) => {{
                if $slot.is_some() {
                    return Err(option.error(concat!("duplicate ", stringify!($slot))));
                }
                $slot = Some(option.value()?.parse()?);
            }};
        }
        let key = option
            .path
            .get_ident()
            .ok_or_else(|| option.error("expected an option name"))?;
        match key.to_string().as_str() {
            "role_name" => field!(role_name),
            "database" => field!(database),
            "authority" => field!(authority),
            "payload" => field!(payload),
            "pages" => field!(pages),
            "read_scope" => field!(read_scope),
            "endpoint" => field!(endpoint),
            "config" => field!(config),
            "status" => field!(status),
            "generator" => field!(generator),
            "write_scope" => field!(write_scope),
            "prefix" => field!(prefix),
            "parent_storage_key" => field!(parent_storage_key),
            _ => return Err(option.error(format!("unknown provisioning option: {key}"))),
        }
        Ok(())
    })
    .parse2(input.parse()?)?;
    // Rust resolves the behavior path, including imported variants and aliases.
    // Generate the row implementation for the supplied layout, then have Rust
    // verify that the selected enum value admits that layout on both targets.
    let hub = config.is_some() || status.is_some();
    if hub && parent_storage_key.is_some() {
        return Err(syn::Error::new(
            role.span(),
            "parent_storage_key requires a shard layout",
        ));
    }
    if !hub && (write_scope.is_some() || generator.is_some()) {
        return Err(syn::Error::new(
            role.span(),
            "write_scope and generator require a hub layout",
        ));
    }
    let layout = if hub { "hub" } else { "shard" };
    let missing = |key| syn::Error::new(role.span(), format!("missing {layout} option: {key}"));
    let database = database.ok_or_else(|| missing("database"))?;
    let authority = authority.ok_or_else(|| missing("authority"))?;
    let field = payload.ok_or_else(|| missing("payload"))?;
    let read = read_scope.unwrap_or_else(|| LitStr::new("read", role.span()));
    let p = quote!(::canic::sharding::provisioning);
    let id = quote!(::ic_db::Ulid);
    let store = quote!(database::store::with);
    let endpoint = endpoint.unwrap_or_else(|| {
        Ident::new(if hub { "register" } else { "provision_user" }, role.span())
    });
    let variant = if hub { quote!(Hub) } else { quote!(Shard) };
    let fallback = role_name.map_or_else(|| quote!(None), |name| quote!(Some(#name)));
    let declaration = quote! {
        /// Application-selected provisioning behavior.
        pub const PROVISIONING_ROLE: #p::ProvisioningRole = #role;
        const _: () = assert!(matches!(PROVISIONING_ROLE, #p::ProvisioningRole::#variant),
            "provisioning behavior differs from the declared storage layout");
        /// Role from the app build, or its standalone Rust declaration.
        pub const APPLICATION_ROLE: Option<&'static str> = match option_env!(concat!("CANIC_APP_ROLE_", env!("CARGO_PKG_NAME"))) {
            Some(configured) => Some(configured),
            None => #fallback,
        };
    };
    if hub {
        let prefix = prefix.map(|value| quote!(prefix = #value,));
        let config = config.ok_or_else(|| missing("config"))?;
        let pages = pages.ok_or_else(|| missing("pages"))?;
        let status = status.ok_or_else(|| missing("status"))?;
        let write = write_scope.unwrap_or_else(|| LitStr::new("write", role.span()));
        let generator = generator.map_or_else(|| quote!(#id::generate_prand), |path| quote!(#path));
        Ok(quote! {
            #declaration
            ::canic::mount_db_role!(hub, database = #database, config = #config, authority = #authority, #prefix pages = #pages, auth_read = #read);
            const _: () = {
                use #p::ProvisioningPayload;
                use #resource as MountedResource;
                use #status as ReservationStatus;
                #[cfg(target_arch = "wasm32")]
                async fn candidate() -> #id { #generator() }
                ::canic::declare_provisioning_row!(hub #row, resource = MountedResource, store = #store,
                    id = (::ic_db::Ulid::to_bytes, ::ic_db::Ulid::from_bytes, candidate), scope = #write,
                    payload = #field: <MountedResource as ProvisioningPayload>::Stored,
                    validate = MountedResource::normalize, export = MountedResource::export,
                    status = (ReservationStatus::Pending, ReservationStatus::Ready));
            };
            #[cfg(target_arch = "wasm32")]
            #[::canic::update(scope = #write, limits = <#resource as #p::Resource>::LIMITS)]
            async fn #endpoint(session: ::canic::auth::protocol::SessionView, input: <#resource as #p::Resource>::Data) -> #p::Progress {
                #p::provision::<#row>(&session, input).await
            }
        })
    } else {
        let prefix = prefix.ok_or_else(|| missing("prefix"))?;
        let parent = parent_storage_key.map(|value| quote!(parent_storage_key = #value,));
        let pages = pages.map(|value| quote!(pages = #value,));
        Ok(quote! {
            #declaration
            ::canic::mount_db_role!(shard, database = #database, #parent authority = #authority, prefix = #prefix, #pages auth_read = #read);
            const _: () = {
                use #p::ProvisioningPayload;
                use #resource as MountedResource;
                ::canic::declare_provisioning_row!(shard #row, resource = MountedResource, store = #store,
                    id = (::ic_db::Ulid::to_bytes, ::ic_db::Ulid::from_bytes), payload = #field, validate = MountedResource::normalize);
            };
            #[cfg(target_arch = "wasm32")]
            #[::canic::update(contract = ::canic::sharding::provisioning::Provision<#resource>, guard = ::canic::topology::require_parent)]
            fn #endpoint(input: #p::Ticket<#row>) -> #p::Binding<[u8; 16]> {
                #p::receive::<#row>(input)
            }
        })
    }
}
