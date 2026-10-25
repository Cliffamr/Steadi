fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "browser_activate_tab",
            "browser_close_tab",
            "browser_create_tab",
            "browser_history",
            "browser_home",
            "browser_navigate",
            "browser_reload",
            "browser_resize_pages",
            "browser_sleep_tab",
            "capture_self_test",
            "reapply_capture_protection",
            "set_click_through",
            "set_window_opacity",
        ]),
    ))
    .expect("failed to run tauri build script");
}
