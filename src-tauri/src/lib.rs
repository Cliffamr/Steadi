// ABOUTME: Starts the Windows-first private overlay browser.
// ABOUTME: Registers bounded browser commands, native controls, and global safety hotkeys.

mod browser;
mod native;

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn shortcut(code: Code) -> Shortcut {
    Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), code)
}

fn emergency_shortcut() -> Shortcut {
    shortcut(Code::KeyH)
}

fn click_through_shortcut() -> Shortcut {
    shortcut(Code::KeyX)
}

fn restore_shortcut() -> Shortcut {
    shortcut(Code::KeyJ)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(browser::BrowserState::default())
        .manage(native::OverlayState::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    if *shortcut == emergency_shortcut() {
                        native::emergency_hide(app).ok();
                    } else if *shortcut == restore_shortcut() {
                        native::restore_main(app).ok();
                    } else if *shortcut == click_through_shortcut() {
                        native::toggle_click_through(app).ok();
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            browser::browser_activate_tab,
            browser::browser_close_tab,
            browser::browser_create_tab,
            browser::browser_history,
            browser::browser_home,
            browser::browser_navigate,
            browser::browser_reload,
            browser::browser_resize_pages,
            browser::browser_sleep_tab,
            native::capture_self_test,
            native::reapply_capture_protection,
            native::set_click_through,
            native::set_window_opacity,
        ])
        .setup(|app| {
            app.global_shortcut().register(emergency_shortcut())?;
            app.global_shortcut().register(restore_shortcut())?;
            app.global_shortcut().register(click_through_shortcut())?;

            if let Some(main) = app.get_webview_window("main") {
                main.show()?;
                let secure = main
                    .set_always_on_top(true)
                    .map_err(|error| error.to_string())
                    .and_then(|_| native::apply_window_opacity(app.handle(), 94).map(|_| ()))
                    .and_then(|_| native::apply_capture_protection(app.handle()).map(|_| ()));
                if let Err(error) = secure {
                    main.hide().ok();
                    return Err(std::io::Error::other(error).into());
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && matches!(event, tauri::WindowEvent::Focused(true))
                && native::apply_capture_protection(window.app_handle()).is_err()
            {
                window.hide().ok();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Veil Browser");
}
