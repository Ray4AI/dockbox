mod commands;
mod config;
mod webdav;
mod webviews;

use std::sync::Mutex;

use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

fn build_tray(app: &tauri::App, autostart: bool) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "开机自启",
        true,
        autostart,
        None::<&str>,
    )?;
    let quit_item = MenuItem::with_id(app, "quit", "退出 DockBox", true, None::<&str>)?;

    let menu = Menu::new(app)?;
    menu.append(&show_item)?;
    menu.append(&autostart_item)?;
    menu.append(&quit_item)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("missing app icon");

    let autostart_item2 = autostart_item.clone();
    TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .tooltip("DockBox")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => commands::show_main_window_impl(app),
            "quit" => app.exit(0),
            "autostart" => {
                let enabled = commands::toggle_autostart(app);
                let _ = autostart_item2.set_checked(enabled);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                commands::toggle_main_window_impl(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            commands::show_main_window_impl(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        commands::toggle_main_window_impl(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let store = config::ConfigStore::load(&handle)?;
            let settings = store.settings_snapshot();
            app.manage(store);
            app.manage(webviews::SvcArgs(webviews::svc_browser_args(
                settings.ignore_cert_errors,
            )));
            app.manage(webviews::OpenSet(Mutex::new(Vec::new())));

            // 主窗口 + 自身 UI 子 webview（填充整个窗口）
            let window = tauri::window::WindowBuilder::new(app, "main")
                .title("DockBox")
                .inner_size(1280.0, 800.0)
                .min_inner_size(860.0, 560.0)
                .build()?;
            let ui = window.add_child(
                tauri::webview::WebviewBuilder::new("ui", tauri::WebviewUrl::App("index.html".into()))
                    .auto_resize()
                    .disable_drag_drop_handler(),
                tauri::LogicalPosition::new(0.0, 0.0),
                window.inner_size().map_err(|e| e.to_string())?,
            )?;
            let _ = ui;
            app.manage(webviews::MainWin(window.clone()));
            eprintln!("[setup] window inner_size = {:?}", window.inner_size());

            // 托盘
            build_tray(app, settings.autostart)?;

            // 关闭按钮行为
            {
                let win = window.clone();
                let handle = handle.clone();
                window.on_window_event(move |e| {
                    if let WindowEvent::CloseRequested { api, .. } = e {
                        let behavior = handle
                            .state::<config::ConfigStore>()
                            .settings_snapshot()
                            .close_to_tray;
                        match behavior.as_str() {
                            "quit" => handle.exit(0),
                            "minimize" => {
                                api.prevent_close();
                                let _ = win.minimize();
                            }
                            _ => {
                                api.prevent_close();
                                let _ = win.hide();
                            }
                        }
                    }
                });
            }

            if settings.start_minimized {
                let _ = window.hide();
            }

            // 全局快捷键
            {
                use tauri_plugin_global_shortcut::GlobalShortcutExt;
                let sc = settings.global_shortcut.trim();
                if !sc.is_empty() {
                    let _ = app.global_shortcut().register(sc);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_state,
            commands::save_data,
            commands::save_settings,
            commands::open_external,
            commands::show_main_window,
            commands::toggle_main_window,
            commands::quit_app,
            commands::webdav_test,
            commands::webdav_set_password,
            commands::webdav_push,
            commands::webdav_pull,
            commands::webview_open,
            commands::webview_activate,
            commands::webview_show,
            commands::webview_hide_all,
            commands::webview_close,
            commands::webview_close_all,
            commands::webview_set_bounds,
            commands::webview_nav,
            commands::webview_set_zoom,
        ])
        .run(tauri::generate_context!())
        .expect("error while running DockBox");
}
