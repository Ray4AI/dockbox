use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::config::{AppData, ConfigStore, LocalSettings};
use crate::{webdav, webviews};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStateResponse {
    pub data: AppData,
    pub settings: LocalSettings,
    pub has_webdav_password: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    pub status: String,
    pub local_rev: u64,
    pub remote_rev: u64,
    pub message: Option<String>,
}

pub fn open_url_external(url: &str) {
    if let Err(e) = tauri_plugin_opener::open_url(url, None::<&str>) {
        eprintln!("open_url failed: {e}");
    }
}

/* ---------------- 基础 ---------------- */

#[tauri::command]
pub fn get_app_state(store: State<'_, ConfigStore>) -> AppStateResponse {
    AppStateResponse {
        data: store.data_snapshot(),
        settings: store.settings_snapshot(),
        has_webdav_password: store.has_webdav_password(),
    }
}

#[tauri::command]
pub fn save_data(store: State<'_, ConfigStore>, data: AppData) -> Result<u64, String> {
    store.save_data(data)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: LocalSettings) -> Result<(), String> {
    // 开机自启
    {
        use tauri_plugin_autostart::ManagerExt;
        let la = app.autolaunch();
        let res = if settings.autostart {
            la.enable()
        } else {
            la.disable()
        };
        if let Err(e) = res {
            eprintln!("autostart error: {e}");
        }
    }
    // 全局快捷键
    {
        use tauri_plugin_global_shortcut::GlobalShortcutExt;
        let gs = app.global_shortcut();
        let _ = gs.unregister_all();
        let sc = settings.global_shortcut.trim();
        if !sc.is_empty() {
            if let Err(e) = gs.register(sc) {
                return Err(format!("快捷键「{sc}」注册失败：{e}"));
            }
        }
    }
    app.state::<ConfigStore>().save_settings(settings)
}

#[tauri::command]
pub fn open_external(url: String) {
    open_url_external(&url);
}

/* ---------------- 窗口 ---------------- */

pub fn show_main_window_impl(app: &AppHandle) {
    let win = app.state::<webviews::MainWin>();
    let _ = win.0.unminimize();
    let _ = win.0.show();
    let _ = win.0.set_focus();
}

pub fn toggle_main_window_impl(app: &AppHandle) {
    let win = app.state::<webviews::MainWin>();
    let w = &win.0;
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show_main_window_impl(app);
    }
}

#[tauri::command]
pub fn show_main_window(app: AppHandle) {
    show_main_window_impl(&app);
}

#[tauri::command]
pub fn toggle_main_window(app: AppHandle) {
    toggle_main_window_impl(&app);
}

#[tauri::command]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

pub fn toggle_autostart(app: &AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    let la = app.autolaunch();
    let new_state = !la.is_enabled().unwrap_or(false);
    let _ = if new_state {
        la.enable()
    } else {
        la.disable()
    };
    let store = app.state::<ConfigStore>();
    let mut s = store.settings_snapshot();
    s.autostart = new_state;
    let _ = store.save_settings(s);
    new_state
}

/* ---------------- WebDAV 同步 ---------------- */

#[tauri::command]
pub async fn webdav_test(app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.state::<ConfigStore>();
        let settings = store.settings_snapshot();
        if settings.webdav.url.trim().is_empty() {
            return Err("尚未填写服务器地址".to_string());
        }
        let password = store.webdav_password()?;
        webdav::test(&settings.webdav, &password)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn webdav_set_password(app: AppHandle, password: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<ConfigStore>().set_webdav_password(&password)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn webdav_push(app: AppHandle, force: bool) -> Result<SyncOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.state::<ConfigStore>();
        let settings = store.settings_snapshot();
        if settings.webdav.url.trim().is_empty() {
            return Err("尚未配置 WebDAV 服务器".to_string());
        }
        let password = store.webdav_password()?;
        let mut data = store.data_snapshot();

        let mut remote_rev = 0u64;
        if let Some(bytes) = webdav::download(&settings.webdav, &password)? {
            let remote: AppData = serde_json::from_slice(&bytes)
                .map_err(|e| format!("远程文件解析失败：{e}"))?;
            if remote.rev > data.rev && !force {
                return Ok(SyncOutcome {
                    status: "conflict".into(),
                    local_rev: data.rev,
                    remote_rev: remote.rev,
                    message: None,
                });
            }
            remote_rev = remote.rev;
        }

        data.rev = data.rev.max(remote_rev) + 1;
        let bytes = serde_json::to_vec_pretty(&data).map_err(|e| e.to_string())?;
        webdav::upload(&settings.webdav, &password, &bytes)?;
        let rev = data.rev;
        store.apply_pushed(data)?;
        Ok(SyncOutcome {
            status: "pushed".into(),
            local_rev: rev,
            remote_rev: rev,
            message: None,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn webdav_pull(app: AppHandle, force: bool) -> Result<SyncOutcome, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = app.state::<ConfigStore>();
        let settings = store.settings_snapshot();
        if settings.webdav.url.trim().is_empty() {
            return Err("尚未配置 WebDAV 服务器".to_string());
        }
        let password = store.webdav_password()?;
        let bytes = webdav::download(&settings.webdav, &password)?
            .ok_or("远程尚无配置文件，可先「上传」")?;
        let remote: AppData = serde_json::from_slice(&bytes)
            .map_err(|e| format!("远程文件解析失败：{e}"))?;
        let data = store.data_snapshot();
        let last_synced = settings.sync_meta.last_synced_rev;

        if remote.rev == data.rev {
            return Ok(SyncOutcome {
                status: "up-to-date".into(),
                local_rev: data.rev,
                remote_rev: remote.rev,
                message: None,
            });
        }
        if remote.rev > data.rev {
            if data.rev > last_synced && !force {
                return Ok(SyncOutcome {
                    status: "conflict".into(),
                    local_rev: data.rev,
                    remote_rev: remote.rev,
                    message: None,
                });
            }
            let rev = remote.rev;
            store.apply_pulled(remote)?;
            Ok(SyncOutcome {
                status: "pulled".into(),
                local_rev: rev,
                remote_rev: rev,
                message: None,
            })
        } else {
            Ok(SyncOutcome {
                status: "local-newer".into(),
                local_rev: data.rev,
                remote_rev: remote.rev,
                message: None,
            })
        }
    })
    .await
    .map_err(|e| e.to_string())?
}

/* ---------------- 子 webview 管理 ---------------- */

#[tauri::command]
pub async fn webview_open(
    app: AppHandle,
    service_id: String,
    url: String,
    zoom: f64,
) -> Result<(), String> {
    // Windows 下创建 webview 不能在同步命令里执行，放到阻塞线程
    tauri::async_runtime::spawn_blocking(move || webviews::open(&app, &service_id, &url, zoom))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub fn webview_activate(
    app: AppHandle,
    service_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    webviews::activate(&app, &service_id, x, y, width, height)
}

#[tauri::command]
pub fn webview_show(
    app: AppHandle,
    service_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    webviews::show(&app, &service_id, x, y, width, height)
}

#[tauri::command]
pub fn webview_hide_all(app: AppHandle) -> Result<(), String> {
    webviews::hide_all(&app)
}

#[tauri::command]
pub fn webview_close(app: AppHandle, service_id: String) -> Result<(), String> {
    webviews::close(&app, &service_id)
}

#[tauri::command]
pub fn webview_close_all(app: AppHandle) -> Result<(), String> {
    webviews::close_all(&app)
}

#[tauri::command]
pub fn webview_set_bounds(
    app: AppHandle,
    service_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    webviews::set_bounds(&app, &service_id, x, y, width, height)
}

#[tauri::command]
pub fn webview_nav(app: AppHandle, service_id: String, action: String) -> Result<(), String> {
    webviews::nav(&app, &service_id, &action)
}

#[tauri::command]
pub fn webview_set_zoom(app: AppHandle, service_id: String, zoom: f64) -> Result<(), String> {
    webviews::set_zoom(&app, &service_id, zoom)
}
