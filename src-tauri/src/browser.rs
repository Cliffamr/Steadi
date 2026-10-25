// ABOUTME: Owns browser tabs as isolated incognito child webviews.
// ABOUTME: Enforces HTTPS-only navigation, bounded tabs, atomic lifecycle, and capture protection.

use serde::Serialize;
use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard};
use tauri::webview::{NewWindowResponse, PageLoadEvent, PermissionResponse};
use tauri::WebviewBuilder;
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Rect, Size, WebviewUrl};

pub const MAX_TABS: usize = 5;
const CHROME_HEIGHT: f64 = 116.0;
const HOME_URL: &str = "https://duckduckgo.com/";

#[derive(Default)]
struct BrowserRegistry {
    tab_ids: HashSet<u32>,
    active_id: Option<u32>,
}

#[derive(Default)]
pub struct BrowserState {
    registry: Mutex<BrowserRegistry>,
    lifecycle: Mutex<()>,
}

impl BrowserState {
    fn lock_lifecycle(&self) -> Result<MutexGuard<'_, ()>, String> {
        self.lifecycle
            .lock()
            .map_err(|_| "Lifecycle tab rusak.".into())
    }

    fn reserve(&self, id: u32) -> Result<(), String> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "Status tab rusak.".to_string())?;
        if registry.tab_ids.contains(&id) {
            return Err("ID tab sudah digunakan.".into());
        }
        if registry.tab_ids.len() >= MAX_TABS {
            return Err(format!("Maksimal {MAX_TABS} tab."));
        }
        registry.tab_ids.insert(id);
        Ok(())
    }

    fn contains(&self, id: u32) -> Result<bool, String> {
        self.registry
            .lock()
            .map(|registry| registry.tab_ids.contains(&id))
            .map_err(|_| "Status tab rusak.".into())
    }

    fn set_active(&self, id: u32) -> Result<(), String> {
        self.registry
            .lock()
            .map(|mut registry| registry.active_id = Some(id))
            .map_err(|_| "Status tab rusak.".into())
    }

    fn is_active(&self, id: u32) -> Result<bool, String> {
        self.registry
            .lock()
            .map(|registry| registry.active_id == Some(id))
            .map_err(|_| "Status tab rusak.".into())
    }

    fn remove(&self, id: u32) -> Result<(), String> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "Status tab rusak.".to_string())?;
        registry.tab_ids.remove(&id);
        if registry.active_id == Some(id) {
            registry.active_id = None;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PageState {
    id: u32,
    url: String,
    title: String,
    loading: bool,
}

fn webview_label(id: u32) -> String {
    format!("page-{id}")
}

fn parse_web_url(value: &str) -> Result<tauri::Url, String> {
    let url = value
        .parse::<tauri::Url>()
        .map_err(|_| "Alamat tidak valid.")?;
    match url.scheme() {
        "https" => Ok(url),
        _ => Err("Hanya alamat HTTPS yang diizinkan.".into()),
    }
}

fn is_web_url(url: &tauri::Url) -> bool {
    url.scheme() == "https"
}

fn emit_page_state(app: &AppHandle, state: PageState) {
    app.emit_to("main", "browser:page-state", state).ok();
}

fn page_bounds(app: &AppHandle) -> Result<(LogicalPosition<f64>, LogicalSize<f64>), String> {
    let main = app
        .get_webview_window("main")
        .ok_or("Jendela utama tidak tersedia.")?;
    let size = main.inner_size().map_err(|error| error.to_string())?;
    let scale = main.scale_factor().map_err(|error| error.to_string())?;
    let width = size.width as f64 / scale;
    let height = (size.height as f64 / scale - CHROME_HEIGHT).max(120.0);
    Ok((
        LogicalPosition::new(0.0, CHROME_HEIGHT),
        LogicalSize::new(width, height),
    ))
}

fn create_page(app: &AppHandle, id: u32, raw_url: &str) -> Result<(), String> {
    let url = parse_web_url(raw_url)?;
    let label = webview_label(id);
    if app.get_webview(&label).is_some() {
        return Err("Tab sudah aktif.".into());
    }

    let main = app
        .get_webview_window("main")
        .ok_or("Jendela utama tidak tersedia.")?;
    let window = main.as_ref().window();
    let (position, size) = page_bounds(app)?;
    let event_app = app.clone();
    let title_app = app.clone();

    let builder = WebviewBuilder::new(label, WebviewUrl::External(url))
        .incognito(true)
        .browser_extensions_enabled(false)
        .devtools(false)
        .on_navigation(is_web_url)
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(|_, _| false)
        .on_permission_request(|_, _| PermissionResponse::Deny)
        .on_page_load(move |_, payload| {
            emit_page_state(
                &event_app,
                PageState {
                    id,
                    url: payload.url().to_string(),
                    title: String::new(),
                    loading: payload.event() == PageLoadEvent::Started,
                },
            );
        })
        .on_document_title_changed(move |_, title| {
            emit_page_state(
                &title_app,
                PageState {
                    id,
                    url: String::new(),
                    title,
                    loading: false,
                },
            );
        });

    window
        .add_child(builder, position, size)
        .map_err(|error| error.to_string())?
        .hide()
        .map_err(|error| error.to_string())
}

fn activate_page(app: &AppHandle, id: u32) -> Result<(), String> {
    let target = webview_label(id);
    let mut found = false;
    for (label, page) in app.webviews() {
        if label.starts_with("page-") {
            if label == target {
                found = true;
                page.show().map_err(|error| error.to_string())?;
                page.set_focus().ok();
            } else {
                page.hide().map_err(|error| error.to_string())?;
            }
        }
    }
    if !found {
        return Err("Tab tidak memiliki halaman aktif.".into());
    }
    Ok(())
}

#[tauri::command]
pub fn browser_create_tab(app: AppHandle, id: u32, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    state.reserve(id)?;
    if let Err(error) = create_page(&app, id, &url).and_then(|_| activate_page(&app, id)) {
        if let Some(page) = app.get_webview(&webview_label(id)) {
            page.close().ok();
        }
        state.remove(id)?;
        return Err(error);
    }
    state.set_active(id)
}

#[tauri::command]
pub fn browser_navigate(app: AppHandle, id: u32, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    app.get_webview(&webview_label(id))
        .ok_or("Tab sedang tidur.")?
        .navigate(parse_web_url(&url)?)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn browser_reload(app: AppHandle, id: u32) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    app.get_webview(&webview_label(id))
        .ok_or("Tab sedang tidur.")?
        .reload()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn browser_history(app: AppHandle, id: u32, delta: i32) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    let page = app
        .get_webview(&webview_label(id))
        .ok_or("Tab sedang tidur.")?;
    let script = if delta < 0 {
        "history.back()"
    } else {
        "history.forward()"
    };
    page.eval(script).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn browser_activate_tab(app: AppHandle, id: u32, url: String) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    if !state.contains(id)? {
        return Err("Tab tidak terdaftar.".into());
    }
    if app.get_webview(&webview_label(id)).is_none() {
        create_page(&app, id, &url)?;
    }
    activate_page(&app, id)?;
    state.set_active(id)
}

#[tauri::command]
pub fn browser_sleep_tab(app: AppHandle, id: u32) -> Result<bool, String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    if state.is_active(id)? {
        return Ok(false);
    }
    if let Some(page) = app.get_webview(&webview_label(id)) {
        page.close().map_err(|error| error.to_string())?;
    }
    Ok(true)
}

#[tauri::command]
pub fn browser_resize_pages(app: AppHandle) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    let (position, size) = page_bounds(&app)?;
    for (label, page) in app.webviews() {
        if label.starts_with("page-") {
            page.set_bounds(Rect {
                position: position.into(),
                size: Size::Logical(size),
            })
            .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn browser_close_tab(app: AppHandle, id: u32) -> Result<(), String> {
    let state = app.state::<BrowserState>();
    let _lifecycle = state.lock_lifecycle()?;
    if let Some(page) = app.get_webview(&webview_label(id)) {
        page.close().map_err(|error| error.to_string())?;
    }
    state.remove(id)
}

#[tauri::command]
pub fn browser_home() -> &'static str {
    HOME_URL
}

#[cfg(test)]
mod tests {
    use super::{BrowserState, MAX_TABS};

    #[test]
    fn registry_enforces_limit_tracks_active_and_releases_ids() {
        let state = BrowserState::default();
        for id in 0..MAX_TABS as u32 {
            assert!(state.reserve(id).is_ok());
        }
        assert!(state.reserve(MAX_TABS as u32).is_err());
        assert!(state.set_active(0).is_ok());
        assert_eq!(state.is_active(0), Ok(true));
        assert!(state.remove(0).is_ok());
        assert_eq!(state.is_active(0), Ok(false));
        assert!(state.reserve(MAX_TABS as u32).is_ok());
    }
}
