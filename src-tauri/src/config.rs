use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf, sync::Mutex};
use tauri::Manager;

/* ---------------- 数据模型（参与 WebDAV 同步） ---------------- */

fn default_zoom() -> f64 {
    1.0
}
fn default_version() -> u32 {
    1
}
fn default_theme() -> String {
    "system".to_string()
}
fn default_suspend() -> u64 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub url: String,
    pub group_id: Option<String>,
    pub icon: Option<String>,
    #[serde(default = "default_zoom")]
    pub zoom: f64,
    pub order: i64,
}

impl Default for Service {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            url: String::new(),
            group_id: None,
            icon: None,
            zoom: 1.0,
            order: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub order: i64,
    pub collapsed: bool,
}

impl Default for Group {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            order: 0,
            collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub updated_at: u64,
    pub services: Vec<Service>,
    pub groups: Vec<Group>,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "默认".into(),
            updated_at: 0,
            services: vec![],
            groups: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prefs {
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_suspend")]
    pub suspend_minutes: u64,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            suspend_minutes: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppData {
    #[serde(default = "default_version")]
    pub version: u32,
    pub rev: u64,
    pub active_profile_id: String,
    pub profiles: std::collections::HashMap<String, Profile>,
    pub prefs: Prefs,
}

impl Default for AppData {
    fn default() -> Self {
        let id = new_id();
        let mut profiles = std::collections::HashMap::new();
        profiles.insert(
            id.clone(),
            Profile {
                id: id.clone(),
                name: "默认".into(),
                updated_at: now_ms(),
                ..Default::default()
            },
        );
        Self {
            version: 1,
            rev: 0,
            active_profile_id: id,
            profiles,
            prefs: Prefs::default(),
        }
    }
}

/* ---------------- 本机设置（不同步） ---------------- */

fn default_remote_path() -> String {
    "/dockbox/dockbox.json".to_string()
}
fn default_shortcut() -> String {
    "Ctrl+Shift+D".to_string()
}
fn default_close_to_tray() -> String {
    "tray".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WebdavSettings {
    pub url: String,
    pub username: String,
    #[serde(default = "default_remote_path")]
    pub remote_path: String,
    pub auto_sync: bool,
}

impl Default for WebdavSettings {
    fn default() -> Self {
        Self {
            url: String::new(),
            username: String::new(),
            remote_path: default_remote_path(),
            auto_sync: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SyncMeta {
    pub last_synced_rev: u64,
    pub last_synced_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LocalSettings {
    #[serde(default = "default_close_to_tray")]
    pub close_to_tray: String,
    pub start_minimized: bool,
    #[serde(default = "default_shortcut")]
    pub global_shortcut: String,
    pub autostart: bool,
    pub ignore_cert_errors: bool,
    pub webdav: WebdavSettings,
    pub sync_meta: SyncMeta,
}

impl Default for LocalSettings {
    fn default() -> Self {
        Self {
            close_to_tray: default_close_to_tray(),
            start_minimized: false,
            global_shortcut: default_shortcut(),
            autostart: false,
            ignore_cert_errors: false,
            webdav: WebdavSettings::default(),
            sync_meta: SyncMeta::default(),
        }
    }
}

/* ---------------- 工具函数 ---------------- */

pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let rand = std::process::id();
    format!("{:x}-{:x}", ms, rand)
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Option<T> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/* ---------------- 存储 ---------------- */

pub struct ConfigStore {
    dir: PathBuf,
    data: Mutex<AppData>,
    settings: Mutex<LocalSettings>,
}

impl ConfigStore {
    pub fn load(app: &tauri::AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|e| e.to_string())?;
        fs::create_dir_all(&dir).ok();

        let mut data: AppData = read_json(&dir.join("data.json")).unwrap_or_default();
        if data.profiles.is_empty() {
            data = AppData::default();
        }
        if !data.profiles.contains_key(&data.active_profile_id) {
            data.active_profile_id = data.profiles.keys().next().cloned().unwrap_or_default();
        }

        let settings: LocalSettings = read_json(&dir.join("settings.json")).unwrap_or_default();

        let store = Self {
            dir,
            data: Mutex::new(data),
            settings: Mutex::new(settings),
        };
        store.flush_data()?;
        store.flush_settings()?;
        Ok(store)
    }

    pub fn data_snapshot(&self) -> AppData {
        self.data.lock().unwrap().clone()
    }

    pub fn settings_snapshot(&self) -> LocalSettings {
        self.settings.lock().unwrap().clone()
    }

    fn flush_data(&self) -> Result<(), String> {
        let data = self.data.lock().unwrap();
        write_json(&self.dir.join("data.json"), &*data)
    }

    fn flush_settings(&self) -> Result<(), String> {
        let s = self.settings.lock().unwrap();
        write_json(&self.dir.join("settings.json"), &*s)
    }

    /// 覆盖数据；内容有变化则 rev+1，返回当前 rev
    pub fn save_data(&self, mut data: AppData) -> Result<u64, String> {
        let mut cur = self.data.lock().unwrap();
        data.rev = cur.rev;
        if serde_json::to_string(&*cur).ok() != serde_json::to_string(&data).ok() {
            data.rev = cur.rev + 1;
        }
        *cur = data;
        let rev = cur.rev;
        drop(cur);
        self.flush_data()?;
        Ok(rev)
    }

    pub fn save_settings(&self, settings: LocalSettings) -> Result<(), String> {
        *self.settings.lock().unwrap() = settings;
        self.flush_settings()
    }

    /// 推送成功后：写入数据 + 更新同步游标
    pub fn apply_pushed(&self, data: AppData) -> Result<(), String> {
        let rev = data.rev;
        *self.data.lock().unwrap() = data;
        self.flush_data()?;
        self.update_sync_meta(rev)
    }

    /// 拉取成功后：写入远程数据 + 更新同步游标
    pub fn apply_pulled(&self, data: AppData) -> Result<(), String> {
        let rev = data.rev;
        *self.data.lock().unwrap() = data;
        self.flush_data()?;
        self.update_sync_meta(rev)
    }

    fn update_sync_meta(&self, rev: u64) -> Result<(), String> {
        {
            let mut s = self.settings.lock().unwrap();
            s.sync_meta.last_synced_rev = rev;
            s.sync_meta.last_synced_at = now_ms();
        }
        self.flush_settings()
    }

    /* WebDAV 密码：保存在系统凭据管理器（keyring） */
    pub fn webdav_password(&self) -> Result<String, String> {
        let entry = keyring::Entry::new("dockbox", "webdav").map_err(|e| e.to_string())?;
        match entry.get_password() {
            Ok(p) => Ok(p),
            Err(keyring::Error::NoEntry) => Ok(String::new()),
            Err(e) => Err(format!("读取凭据失败：{e}")),
        }
    }

    pub fn set_webdav_password(&self, password: &str) -> Result<(), String> {
        let entry = keyring::Entry::new("dockbox", "webdav").map_err(|e| e.to_string())?;
        if password.is_empty() {
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(format!("删除凭据失败：{e}")),
            }
        } else {
            entry.set_password(password).map_err(|e| e.to_string())
        }
    }

    pub fn has_webdav_password(&self) -> bool {
        !self.webdav_password().unwrap_or_default().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> ConfigStore {
        let dir = std::env::temp_dir().join(format!("dockbox-test-{}", new_id()));
        fs::create_dir_all(&dir).unwrap();
        ConfigStore {
            dir,
            data: Mutex::new(AppData::default()),
            settings: Mutex::new(LocalSettings::default()),
        }
    }

    #[test]
    fn default_data_has_one_profile() {
        let data = AppData::default();
        assert_eq!(data.profiles.len(), 1);
        assert!(data.profiles.contains_key(&data.active_profile_id));
        assert_eq!(data.prefs.theme, "system");
    }

    #[test]
    fn save_data_bumps_rev_only_on_change() {
        let store = temp_store();
        let mut data = store.data_snapshot();
        assert_eq!(data.rev, 0);

        // 无变化 → rev 不变
        let rev = store.save_data(data.clone()).unwrap();
        assert_eq!(rev, 0);

        // 有变化 → rev +1
        data.profiles.get_mut(&data.active_profile_id).unwrap().name = "改名".into();
        let rev = store.save_data(data.clone()).unwrap();
        assert_eq!(rev, 1);

        // 再次无变化 → rev 不变
        let rev = store.save_data(data).unwrap();
        assert_eq!(rev, 1);
    }

    #[test]
    fn serde_uses_camel_case() {
        let s = Service {
            id: "x".into(),
            name: "n".into(),
            url: "http://a".into(),
            group_id: Some("g".into()),
            icon: None,
            zoom: 1.0,
            order: 1,
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("\"groupId\":\"g\""), "{json}");
        assert!(json.contains("\"zoom\":1.0") || json.contains("\"zoom\":1"), "{json}");

        // 缺字段也能反序列化（向前兼容）
        let back: Service = serde_json::from_str(r#"{"id":"x","name":"n","url":"u"}"#).unwrap();
        assert_eq!(back.zoom, 1.0);
        assert!(back.group_id.is_none());
    }
}
