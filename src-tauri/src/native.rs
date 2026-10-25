// ABOUTME: Applies Windows overlay controls and verifies capture-exclusion state.
// ABOUTME: Keeps click-through, opacity, and fail-closed emergency behavior in the trusted core.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub struct OverlayState {
    click_through: AtomicBool,
    opacity: AtomicU8,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            click_through: AtomicBool::new(false),
            opacity: AtomicU8::new(94),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSelfTest {
    platform: &'static str,
    protected: bool,
    affinity: u32,
    detail: String,
}

fn main_window(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    app.get_webview_window("main")
        .ok_or_else(|| "Jendela utama tidak tersedia.".into())
}

pub fn apply_capture_protection(app: &AppHandle) -> Result<u32, String> {
    let window = main_window(app)?;

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::GetLastError;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowDisplayAffinity, SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE,
        };

        let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
        if unsafe { SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE) } == 0 {
            let code = unsafe { GetLastError() };
            return Err(format!(
                "Windows menolak WDA_EXCLUDEFROMCAPTURE (error {code})."
            ));
        }

        let mut affinity = 0u32;
        if unsafe { GetWindowDisplayAffinity(hwnd, &mut affinity) } == 0 {
            let code = unsafe { GetLastError() };
            return Err(format!(
                "Windows gagal membaca display affinity (error {code})."
            ));
        }
        if affinity != WDA_EXCLUDEFROMCAPTURE {
            return Err(format!("Display affinity tidak aman: 0x{affinity:08x}."));
        }
        return Ok(affinity);
    }

    #[cfg(not(target_os = "windows"))]
    {
        window
            .set_content_protected(true)
            .map_err(|error| error.to_string())?;
        Ok(0)
    }
}

pub fn set_click_through_inner(app: &AppHandle, enabled: bool) -> Result<(), String> {
    let window = main_window(app)?;
    let previous = app
        .state::<OverlayState>()
        .click_through
        .load(Ordering::Relaxed);
    window
        .set_ignore_cursor_events(enabled)
        .map_err(|error| error.to_string())?;
    let opacity = app.state::<OverlayState>().opacity.load(Ordering::Relaxed);
    if let Err(error) =
        apply_window_opacity(app, opacity).and_then(|_| apply_capture_protection(app))
    {
        window.set_ignore_cursor_events(previous).ok();
        window.hide().ok();
        return Err(error);
    }
    app.state::<OverlayState>()
        .click_through
        .store(enabled, Ordering::Relaxed);
    app.emit_to("main", "overlay:click-through", enabled).ok();
    Ok(())
}

pub fn toggle_click_through(app: &AppHandle) -> Result<bool, String> {
    let enabled = !app
        .state::<OverlayState>()
        .click_through
        .load(Ordering::Relaxed);
    set_click_through_inner(app, enabled)?;
    Ok(enabled)
}

pub fn emergency_hide(app: &AppHandle) -> Result<(), String> {
    let window = main_window(app)?;
    let visible = window.is_visible().map_err(|error| error.to_string())?;
    if visible {
        window.hide().map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn restore_main(app: &AppHandle) -> Result<(), String> {
    let window = main_window(app)?;
    window.show().map_err(|error| error.to_string())?;
    let opacity = app.state::<OverlayState>().opacity.load(Ordering::Relaxed);
    let secure = window
        .set_always_on_top(true)
        .map_err(|error| error.to_string())
        .and_then(|_| apply_window_opacity(app, opacity).map(|_| ()))
        .and_then(|_| apply_capture_protection(app).map(|_| ()))
        .and_then(|_| window.set_focus().map_err(|error| error.to_string()));
    if let Err(error) = secure {
        window.hide().ok();
        return Err(error);
    }
    Ok(())
}

#[tauri::command]
pub fn set_click_through(app: AppHandle, enabled: bool) -> Result<(), String> {
    set_click_through_inner(&app, enabled)
}

pub fn apply_window_opacity(app: &AppHandle, percent: u8) -> Result<u8, String> {
    let percent = percent.clamp(30, 100);

    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE,
            LWA_ALPHA, WS_EX_LAYERED,
        };

        let window = main_window(app)?;
        let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
        let alpha = ((u16::from(percent) * 255) / 100) as u8;
        unsafe {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
            if SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) == 0 {
                return Err("Windows menolak perubahan transparansi.".into());
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    let _ = app;

    Ok(percent)
}

#[tauri::command]
pub fn set_window_opacity(app: AppHandle, percent: u8) -> Result<u8, String> {
    let percent = apply_window_opacity(&app, percent)?;
    app.state::<OverlayState>()
        .opacity
        .store(percent, Ordering::Relaxed);
    if let Err(error) = apply_capture_protection(&app) {
        main_window(&app)?.hide().ok();
        return Err(error);
    }
    Ok(percent)
}

#[tauri::command]
pub fn reapply_capture_protection(app: AppHandle) -> Result<(), String> {
    apply_capture_protection(&app).map(|_| ())
}

#[tauri::command]
pub fn capture_self_test(app: AppHandle) -> Result<CaptureSelfTest, String> {
    let affinity = apply_capture_protection(&app)?;

    #[cfg(target_os = "windows")]
    return Ok(CaptureSelfTest {
        platform: "Windows",
        protected: true,
        affinity,
        detail: "WDA_EXCLUDEFROMCAPTURE aktif. Lanjutkan pemeriksaan preview tiap aplikasi rapat."
            .into(),
    });

    #[cfg(not(target_os = "windows"))]
    Ok(CaptureSelfTest {
        platform: std::env::consts::OS,
        protected: false,
        affinity,
        detail: "Build ini ditujukan untuk Windows 10 2004 atau lebih baru.".into(),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn opacity_bounds_match_the_ui_contract() {
        assert_eq!(20u8.clamp(30, 100), 30);
        assert_eq!(75u8.clamp(30, 100), 75);
        assert_eq!(120u8.clamp(30, 100), 100);
    }
}
