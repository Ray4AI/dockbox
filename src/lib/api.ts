import { invoke } from "@tauri-apps/api/core";
import type { AppData, AppState, Bounds, LocalSettings, SyncOutcome } from "./types";

export function getAppState(): Promise<AppState> {
  return invoke<AppState>("get_app_state");
}

/** 返回新的 rev */
export function saveData(data: AppData): Promise<number> {
  return invoke<number>("save_data", { data });
}

export function saveSettings(settings: LocalSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function openExternal(url: string): Promise<void> {
  await invoke("open_external", { url });
}

export function toggleWindow(): Promise<void> {
  return invoke("toggle_main_window");
}

export function showWindow(): Promise<void> {
  return invoke("show_main_window");
}

export function quitApp(): Promise<void> {
  return invoke("quit_app");
}

/* ---------- WebDAV / 同步 ---------- */

export function webdavTest(): Promise<string> {
  return invoke<string>("webdav_test");
}

export function webdavSetPassword(password: string): Promise<void> {
  return invoke("webdav_set_password", { password });
}

export function webdavPush(force: boolean): Promise<SyncOutcome> {
  return invoke<SyncOutcome>("webdav_push", { force });
}

export function webdavPull(force: boolean): Promise<SyncOutcome> {
  return invoke<SyncOutcome>("webdav_pull", { force });
}

/* ---------- 子 webview 管理 ---------- */

export function webviewOpen(serviceId: string, url: string, zoom: number): Promise<void> {
  return invoke("webview_open", { serviceId, url, zoom });
}

export function webviewActivate(serviceId: string, bounds: Bounds): Promise<void> {
  return invoke("webview_activate", { serviceId, ...bounds });
}

export function webviewHideAll(): Promise<void> {
  return invoke("webview_hide_all");
}

export function webviewShow(serviceId: string, bounds: Bounds): Promise<void> {
  return invoke("webview_show", { serviceId, ...bounds });
}

export function webviewClose(serviceId: string): Promise<void> {
  return invoke("webview_close", { serviceId });
}

export function webviewCloseAll(): Promise<void> {
  return invoke("webview_close_all");
}

export function webviewSetBounds(serviceId: string, bounds: Bounds): Promise<void> {
  return invoke("webview_set_bounds", { serviceId, ...bounds });
}

export function webviewNav(serviceId: string, action: "back" | "forward" | "reload"): Promise<void> {
  return invoke("webview_nav", { serviceId, action });
}

export function webviewSetZoom(serviceId: string, zoom: number): Promise<void> {
  return invoke("webview_set_zoom", { serviceId, zoom });
}
