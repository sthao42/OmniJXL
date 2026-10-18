use std::fs;
use std::path::Path;

#[test]
fn test_tauri_conf_ensures_offline_standalone_frontend() {
    let conf_path = Path::new("tauri.conf.json");
    assert!(
        conf_path.exists(),
        "tauri.conf.json must exist in src-tauri"
    );

    let conf_str = fs::read_to_string(conf_path).expect("read tauri.conf.json");
    let conf: serde_json::Value =
        serde_json::from_str(&conf_str).expect("valid JSON in tauri.conf.json");

    let build_obj = conf.get("build").expect("build section in tauri.conf.json");

    // Regression prevention: Ensure devUrl is NOT set in default config to prevent
    // "localhost refused to connect" error when omnijxl.exe is run standalone
    let dev_url = build_obj.get("devUrl");
    assert!(
        dev_url.is_none() || dev_url.unwrap().is_null(),
        "devUrl must not be active in standalone config; found {:?}",
        dev_url
    );

    // Ensure frontendDist is configured to ../build
    let frontend_dist = build_obj
        .get("frontendDist")
        .expect("frontendDist in tauri.conf.json")
        .as_str()
        .expect("frontendDist string");
    assert_eq!(
        frontend_dist, "../build",
        "frontendDist must point to the static build directory"
    );

    // Verify ../build exists and contains index.html
    let build_dir = Path::new(frontend_dist);
    assert!(
        build_dir.exists(),
        "Static build directory {:?} must exist",
        build_dir
    );
    let index_html = build_dir.join("index.html");
    assert!(
        index_html.exists(),
        "index.html must exist in build directory for standalone webview startup"
    );

    let html_content = fs::read_to_string(&index_html).expect("read index.html");
    assert!(
        html_content.contains("<!doctype html>") || html_content.contains("<html"),
        "index.html must contain valid HTML content"
    );
}

#[test]
fn test_rust_runtime_version_is_1_99_1() {
    // 1. Check Cargo.toml MSRV specification
    let cargo_toml_str = fs::read_to_string("Cargo.toml").expect("read Cargo.toml");
    assert!(
        cargo_toml_str.contains("rust-version = \"1.99.1\""),
        "Cargo.toml must enforce rust-version = \"1.99.1\""
    );

    // 2. Check rust-toolchain.toml pinning
    let toolchain_path = Path::new("../rust-toolchain.toml");
    assert!(
        toolchain_path.exists(),
        "rust-toolchain.toml must exist at project root"
    );
    let toolchain_str = fs::read_to_string(toolchain_path).expect("read rust-toolchain.toml");
    assert!(
        toolchain_str.contains("channel = \"1.99.1\""),
        "rust-toolchain.toml must pin channel = \"1.99.1\""
    );

    // 3. Check Sidebar UI text explicitly displays Rust 1.99.1
    let sidebar_path = Path::new("../src/lib/components/Sidebar.svelte");
    assert!(sidebar_path.exists(), "Sidebar.svelte must exist");
    let sidebar_str = fs::read_to_string(sidebar_path).expect("read Sidebar.svelte");
    assert!(
        sidebar_str.contains("Tauri v2 + Rust 1.99.1"),
        "Sidebar UI text must display 'Tauri v2 + Rust 1.99.1'"
    );
}

#[test]
fn test_app_icons_exist_and_are_valid() {
    let ico_path = Path::new("icons/icon.ico");
    assert!(ico_path.exists(), "icon.ico must exist");
    assert!(
        fs::metadata(ico_path).unwrap().len() > 1000,
        "icon.ico must be non-empty"
    );

    let png_path = Path::new("icons/icon.png");
    assert!(png_path.exists(), "icon.png must exist");
    assert!(
        fs::metadata(png_path).unwrap().len() > 1000,
        "icon.png must be non-empty"
    );

    let favicon_path = Path::new("../static/favicon.png");
    assert!(favicon_path.exists(), "static/favicon.png must exist");
    assert!(
        fs::metadata(favicon_path).unwrap().len() > 500,
        "favicon.png must be non-empty"
    );

    let app_icon_path = Path::new("../static/app-icon.png");
    assert!(app_icon_path.exists(), "static/app-icon.png must exist");
    assert!(
        fs::metadata(app_icon_path).unwrap().len() > 500,
        "app-icon.png must be non-empty"
    );
}
