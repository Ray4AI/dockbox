use std::sync::Mutex;
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, Rect, WebviewUrl};

use crate::commands;

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect {
        position: LogicalPosition::new(x, y).into(),
        size: LogicalSize::new(w, h).into(),
    }
}

/// 主窗口句柄（窗口由 setup 创建，托管以便命令中使用）
pub struct MainWin(pub tauri::Window);

/// 子 webview 的 WebView2 浏览器参数（进程内保持不变）
pub struct SvcArgs(pub String);

/// 当前打开的子 webview（serviceId 列表）
pub struct OpenSet(pub Mutex<Vec<String>>);

pub fn svc_browser_args(ignore_cert_errors: bool) -> String {
    // wry 默认参数，追加证书忽略开关；全部子 webview 保持一致
    let mut args = String::from("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection");
    if ignore_cert_errors {
        args.push_str(" --ignore-certificate-errors");
    }
    args
}

fn label(id: &str) -> String {
    format!("svc-{id}")
}

fn window(app: &AppHandle) -> Result<tauri::Window, String> {
    let win = app.state::<MainWin>();
    Ok(win.0.clone())
}

fn mark_open(app: &AppHandle, id: &str) {
    let set_state = app.state::<OpenSet>();
    let mut set = set_state.0.lock().unwrap();
    if !set.iter().any(|x| x == id) {
        set.push(id.to_string());
    }
}

fn mark_closed(app: &AppHandle, id: &str) {
    let set_state = app.state::<OpenSet>();
    let mut set = set_state.0.lock().unwrap();
    set.retain(|x| x != id);
}

pub fn open(app: &AppHandle, id: &str, url: &str, zoom: f64) -> Result<(), String> {
    let lbl = label(id);
    if app.get_webview(&lbl).is_some() {
        mark_open(app, id);
        return Ok(());
    }

    let window = window(app)?;
    let parsed = url::Url::parse(url).map_err(|e| format!("无效的地址：{e}"))?;
    let args = app.state::<SvcArgs>().0.clone();
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("svc-webview");
    std::fs::create_dir_all(&data_dir).ok();

    let builder = tauri::webview::WebviewBuilder::new(lbl, WebviewUrl::External(parsed))
        .additional_browser_args(&args)
        .data_directory(data_dir)
        .on_new_window(|u, _features| {
            commands::open_url_external(u.as_str());
            tauri::webview::NewWindowResponse::Deny
        });

    let webview = window
        .add_child(
            builder,
            LogicalPosition::new(0.0, 0.0),
            LogicalSize::new(200.0, 150.0),
        )
        .map_err(|e| format!("创建子页面失败：{e}"))?;
    eprintln!("[webview] created {id} (hidden)");

    if (zoom - 1.0).abs() > 0.001 {
        let _ = webview.set_zoom(zoom);
    }
    // 创建后立即隐藏，由 activate 决定显示
    let _ = webview.hide();
    mark_open(app, id);
    Ok(())
}

pub fn activate(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    eprintln!("[webview] activate {id} bounds=({x},{y},{w},{h})");
    let open_ids = app.state::<OpenSet>().0.lock().unwrap().clone();
    for oid in open_ids {
        if let Some(wv) = app.get_webview(&label(&oid)) {
            if oid == id {
                let r0 = wv.set_bounds(rect(x, y, w, h));
                let r3 = wv.show();
                eprintln!("[webview] set_bounds={r0:?} show={r3:?}");
                r0.map_err(|e| e.to_string())?;
                r3.map_err(|e| e.to_string())?;
                let _ = wv.set_focus();
            } else {
                let _ = wv.hide();
            }
        }
    }
    Ok(())
}

pub fn show(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    eprintln!("[webview] show {id} bounds=({x},{y},{w},{h})");
    if let Some(wv) = app.get_webview(&label(id)) {
        wv.set_bounds(rect(x, y, w, h)).map_err(|e| e.to_string())?;
        wv.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn hide_all(app: &AppHandle) -> Result<(), String> {
    eprintln!("[webview] hide_all");
    let open_ids = app.state::<OpenSet>().0.lock().unwrap().clone();
    for oid in open_ids {
        if let Some(wv) = app.get_webview(&label(&oid)) {
            let _ = wv.hide();
        }
    }
    Ok(())
}

pub fn set_bounds(app: &AppHandle, id: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&label(id)) {
        wv.set_bounds(rect(x, y, w, h)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn close(app: &AppHandle, id: &str) -> Result<(), String> {
    eprintln!("[webview] close {id}");
    if let Some(wv) = app.get_webview(&label(id)) {
        let _ = wv.close();
    }
    mark_closed(app, id);
    Ok(())
}

pub fn close_all(app: &AppHandle) -> Result<(), String> {
    let open_ids = app.state::<OpenSet>().0.lock().unwrap().clone();
    for oid in open_ids {
        close(app, &oid)?;
    }
    Ok(())
}

pub fn nav(app: &AppHandle, id: &str, action: &str) -> Result<(), String> {
    eprintln!("[webview] nav {id} {action}");
    let js = match action {
        "back" => "history.back()",
        "forward" => "history.forward()",
        "reload" => "location.reload()",
        _ => return Err(format!("未知操作：{action}")),
    };
    if let Some(wv) = app.get_webview(&label(id)) {
        wv.eval(js).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn set_zoom(app: &AppHandle, id: &str, zoom: f64) -> Result<(), String> {
    if let Some(wv) = app.get_webview(&label(id)) {
        wv.set_zoom(zoom).map_err(|e| e.to_string())?;
    }
    Ok(())
}
