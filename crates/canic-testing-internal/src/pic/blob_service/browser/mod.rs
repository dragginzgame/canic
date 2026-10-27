//! Execute retained generated Blob codecs against installed tenant endpoints.
//! PocketIC supplies transport; this does not qualify a browser UI or provider.

use canic_blob_service::dto::tenant::TenantScope;
use canic_host::frontend::ops::compiler::{BrowserCompiler, compile};
use ic_platform::{pocket_ic::PocketIc, principal::ExternalPrincipal};
use std::{fs, path::Path, process::Command};

pub(super) fn qualify(
    pic: &PocketIc,
    wasm: &Path,
    service: ExternalPrincipal,
    operator: ExternalPrincipal,
    stranger: ExternalPrincipal,
    scope: TenantScope,
) {
    let directory = std::env::temp_dir().join(format!(
        "blob-browser-{}-{}",
        std::process::id(),
        wasm.file_stem().unwrap().to_str().unwrap()
    ));
    fs::create_dir(&directory).unwrap();
    let compiler = BrowserCompiler {
        node: std::env::var_os("CANIC_FRONTEND_NODE")
            .expect("prepared Node")
            .into(),
        typescript: std::env::var_os("CANIC_FRONTEND_TYPESCRIPT")
            .expect("prepared TypeScript")
            .into(),
    };
    let bindings = super::schema::qualify_retained(wasm);
    let bundle = compile(
        &directory.join("cache"),
        &directory.join("bundle.json"),
        &compiler,
        &bindings,
    )
    .unwrap();
    for (name, contents) in bundle.files {
        fs::write(directory.join(name), contents).unwrap();
    }
    let run = |action: &str| {
        let script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/wire_composition/installed.mjs");
        let output = Command::new(&compiler.node)
            .arg(script)
            .arg(action)
            .arg(&directory)
            .arg(serde_json::to_string(service.as_slice()).unwrap())
            .arg(serde_json::to_string(scope.tenant.as_slice()).unwrap())
            .arg(scope.namespace.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    run("query");
    let request = fs::read(directory.join("query.request")).unwrap();
    let reply = pic
        .query_call(service, operator, "blob_tenant", request.clone())
        .unwrap();
    fs::write(directory.join("query.response"), reply).unwrap();
    let denied = pic
        .query_call(service, stranger, "blob_tenant", request)
        .unwrap();
    fs::write(directory.join("query-denied.response"), denied).unwrap();
    let wrong_scope = fs::read(directory.join("query-binding.request")).unwrap();
    let refused = pic
        .query_call(service, operator, "blob_tenant", wrong_scope)
        .unwrap();
    fs::write(directory.join("query-binding.response"), refused).unwrap();
    run("update");
    refuse_invalid_updates(pic, service, operator, &directory);
    let request = fs::read(directory.join("update.request")).unwrap();
    let denied = pic
        .update_call(service, stranger, "blob_update_tenant", request.clone())
        .unwrap();
    fs::write(directory.join("update-denied.response"), denied).unwrap();
    for name in ["update.response", "replay.response"] {
        let reply = pic
            .update_call(service, operator, "blob_update_tenant", request.clone())
            .unwrap();
        fs::write(directory.join(name), reply).unwrap();
    }
    run("verify");
}

fn refuse_invalid_updates(
    pic: &PocketIc,
    service: ExternalPrincipal,
    operator: ExternalPrincipal,
    directory: &Path,
) {
    let stable = pic.get_stable_memory(service);
    let request = fs::read(directory.join("update-binding.request")).unwrap();
    let reply = pic
        .update_call(service, operator, "blob_update_tenant", request)
        .unwrap();
    fs::write(directory.join("update-binding.response"), reply).unwrap();
    assert_eq!(pic.get_stable_memory(service), stable);
    let original = fs::read(directory.join("update.request")).unwrap();
    let mut trailing = original.clone();
    trailing.push(0);
    for malformed in [trailing, original[..original.len() - 1].to_vec()] {
        assert!(
            pic.update_call(service, operator, "blob_update_tenant", malformed)
                .is_err()
        );
        assert_eq!(pic.get_stable_memory(service), stable);
    }
}
