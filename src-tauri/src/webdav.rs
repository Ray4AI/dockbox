use base64::Engine;

use crate::config::WebdavSettings;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(15))
        .build()
}

fn auth(user: &str, pass: &str) -> String {
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
    format!("Basic {token}")
}

pub fn full_url(settings: &WebdavSettings) -> String {
    let base = settings.url.trim().trim_end_matches('/');
    let mut path = settings.remote_path.trim().to_string();
    if path.is_empty() {
        path = "/dockbox/dockbox.json".into();
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    format!("{base}{path}")
}

/// 测试连接：200/404 都算成功（404 = 远程尚无文件）
pub fn test(settings: &WebdavSettings, password: &str) -> Result<String, String> {
    let url = full_url(settings);
    let resp = agent()
        .get(&url)
        .set("Authorization", &auth(&settings.username, password))
        .call();
    match resp {
        Ok(r) => Ok(format!("连接成功（HTTP {}），远程文件可读写", r.status())),
        Err(ureq::Error::Status(404, _)) => {
            Ok("连接成功（远程尚无配置文件，同步时会自动创建）".into())
        }
        Err(ureq::Error::Status(401, _)) => Err("认证失败（HTTP 401）：请检查用户名 / 应用密码".into()),
        Err(ureq::Error::Status(s, _)) => Err(format!("服务器返回 HTTP {s}")),
        Err(e) => Err(format!("连接失败：{e}")),
    }
}

pub fn download(settings: &WebdavSettings, password: &str) -> Result<Option<Vec<u8>>, String> {
    let url = full_url(settings);
    let resp = agent()
        .get(&url)
        .set("Authorization", &auth(&settings.username, password))
        .call();
    match resp {
        Ok(r) => {
            let mut buf = Vec::new();
            let mut reader = r.into_reader();
            std::io::Read::read_to_end(&mut reader, &mut buf)
                .map_err(|e| format!("读取远程文件失败：{e}"))?;
            Ok(Some(buf))
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(ureq::Error::Status(401, _)) => {
            Err("认证失败（HTTP 401）：请检查用户名 / 应用密码".into())
        }
        Err(ureq::Error::Status(s, _)) => Err(format!("下载失败：HTTP {s}")),
        Err(e) => Err(format!("下载失败：{e}")),
    }
}

pub fn upload(settings: &WebdavSettings, password: &str, bytes: &[u8]) -> Result<(), String> {
    let url = full_url(settings);
    let resp = agent()
        .put(&url)
        .set("Authorization", &auth(&settings.username, password))
        .set("Content-Type", "application/json")
        .send_bytes(bytes);
    match resp {
        Ok(_) => Ok(()),
        Err(ureq::Error::Status(401, _)) => {
            Err("认证失败（HTTP 401）：请检查用户名 / 应用密码".into())
        }
        Err(ureq::Error::Status(s, _)) => Err(format!("上传失败：HTTP {s}")),
        Err(e) => Err(format!("上传失败：{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    /// 极简 WebDAV 测试服务器：Basic 认证 + GET/PUT/404
    fn spawn_server(
        user: &str,
        pass: &str,
    ) -> (String, Arc<Mutex<Option<Vec<u8>>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let file: Arc<Mutex<Option<Vec<u8>>>> = Arc::new(Mutex::new(None));
        let file2 = file.clone();
        let want_auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"))
        );

        let handle = std::thread::spawn(move || {
            for stream in listener.incoming().take(20) {
                let mut s = stream.unwrap();
                let mut buf = Vec::new();
                let mut tmp = [0u8; 2048];
                // 读到头部结束
                loop {
                    let n = s.read(&mut tmp).unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                    if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let head = String::from_utf8_lossy(&buf).to_string();
                let first = head.lines().next().unwrap_or("").to_string();
                let authed = head
                    .lines()
                    .find(|l| l.to_lowercase().starts_with("authorization:"))
                    .map(|l| l[15..].trim() == want_auth)
                    .unwrap_or(false);

                if !authed {
                    s.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n")
                        .ok();
                    continue;
                }

                if first.starts_with("GET") {
                    let body = file2.lock().unwrap().clone();
                    match body {
                        Some(b) => {
                            let head = format!(
                                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n",
                                b.len()
                            );
                            s.write_all(head.as_bytes()).ok();
                            s.write_all(&b).ok();
                        }
                        None => {
                            s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n")
                                .ok();
                        }
                    }
                } else if first.starts_with("PUT") {
                    // 读 body
                    let clen = head
                        .lines()
                        .find(|l| l.to_lowercase().starts_with("content-length:"))
                        .and_then(|l| l[15..].trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    let header_end = head.find("\r\n\r\n").map(|i| i + 4).unwrap_or(buf.len());
                    let mut body = buf[header_end.min(buf.len())..].to_vec();
                    while body.len() < clen {
                        let n = s.read(&mut tmp).unwrap_or(0);
                        if n == 0 {
                            break;
                        }
                        body.extend_from_slice(&tmp[..n]);
                    }
                    *file2.lock().unwrap() = Some(body);
                    s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                        .ok();
                } else {
                    s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n")
                        .ok();
                }
            }
        });

        (format!("http://{addr}"), file, handle)
    }

    fn settings(base: &str) -> WebdavSettings {
        WebdavSettings {
            url: format!("{base}/dav"),
            username: "ray".into(),
            remote_path: "/dockbox/dockbox.json".into(),
            auto_sync: true,
        }
    }

    #[test]
    fn full_url_joins_base_and_path() {
        let mut s = settings("https://dav.example.com");
        s.url = "https://dav.example.com/dav/".into();
        assert_eq!(
            full_url(&s),
            "https://dav.example.com/dav/dockbox/dockbox.json"
        );
        s.url = "https://dav.example.com/dav".into();
        s.remote_path = "dockbox.json".into();
        assert_eq!(full_url(&s), "https://dav.example.com/dav/dockbox.json");
    }

    #[test]
    fn connect_upload_download_roundtrip() {
        let (base, file, _h) = spawn_server("ray", "secret");
        let s = settings(&base);

        // 远程无文件 → 连接成功提示
        let msg = test(&s, "secret").unwrap();
        assert!(msg.contains("尚无配置文件"), "msg = {msg}");
        assert!(download(&s, "secret").unwrap().is_none());

        // 上传后可下载回相同内容
        let payload = br#"{"version":1,"rev":5}"#.to_vec();
        upload(&s, "secret", &payload).unwrap();
        assert_eq!(download(&s, "secret").unwrap().unwrap(), payload);
        assert_eq!(file.lock().unwrap().clone().unwrap(), payload);

        // 文件存在时连接测试也应成功
        let msg = test(&s, "secret").unwrap();
        assert!(msg.contains("连接成功"), "msg = {msg}");
    }

    #[test]
    fn wrong_password_is_auth_error() {
        let (base, _f, _h) = spawn_server("ray", "secret");
        let s = settings(&base);
        let err = test(&s, "wrong").unwrap_err();
        assert!(err.contains("认证失败"), "err = {err}");
        let err = download(&s, "wrong").unwrap_err();
        assert!(err.contains("认证失败"), "err = {err}");
    }
}
