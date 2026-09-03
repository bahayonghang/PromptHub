//! Locks the webview CSP string and the invoke_handler surface.

const LOCKED_CSP: &str = "default-src 'self'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline'";

const RETIRED_HANDLER_FNS: &[&str] = &[
    "ai_request",
    "ai_stream",
    "ai_cancel",
    "sync_webdav_upload",
    "sync_webdav_download",
    "sync_webdav_stat",
    "sync_webdav_ensure_dir",
    "sync_s3_upload",
    "sync_s3_download",
    "sync_s3_stat",
];

fn generate_handler_source() -> &'static str {
    const LIB: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    let start = LIB
        .find("tauri::generate_handler![")
        .expect("generate_handler! must be present in lib.rs");
    let relative_end = LIB[start..]
        .find(']')
        .expect("generate_handler! must terminate");
    &LIB[start..=start + relative_end]
}

#[test]
fn csp_is_the_locked_strict_policy() {
    let conf: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tauri.conf.json"
    )))
    .unwrap();
    let csp = conf["app"]["security"]["csp"].as_str();
    assert_eq!(csp, Some(LOCKED_CSP));
}

#[test]
fn invoke_handler_omits_retired_network_commands() {
    let handler = generate_handler_source();
    for name in RETIRED_HANDLER_FNS {
        assert!(
            !handler.contains(name),
            "{name} must not be registered in generate_handler!"
        );
    }
    assert!(
        handler.contains("sync_webdav_test"),
        "webdav.test must remain registered"
    );
    assert!(
        handler.contains("sync_s3_test"),
        "s3.test must remain registered"
    );
}
