# DockBox

<p align="center"><b>轻量级 Docker Web 工具面板</b> —— 左侧标签、右侧页面、托盘驻留、WebDAV 同步</p>

<p align="center">
  <a href="https://github.com/Ray4AI/dockbox">GitHub</a> ·
  <a href="https://github.com/Ray4AI/dockbox/actions">构建产物（Actions → Artifacts）</a>
</p>

DockBox 是一个 Windows 上的极轻量"内部浏览器"，专门用来管理一堆 Docker 小工具的 Web 页面
（Portainer、qBittorrent、Jellyfin、Grafana、Home Assistant……）。
基于 **Tauri 2 + WebView2**，不打包浏览器内核，安装包约 10MB，空载内存占用不到 100MB。

## 功能

- **左侧标签 / 右侧页面**：每个服务一个独立 webview，切换秒开、不重载、登录态保持
- **多配置（Profile）**：家里 NAS / 公司 / 实验室多套服务列表，一键切换
- **分组管理**：服务归组、折叠、拖拽排序、搜索过滤
- **WebDAV 同步**：配置（含全部 Profile）同步到 WebDAV，换电脑即刻可用
  - 自动同步（改动后上传、启动时拉取）+ 手动上传/下载
  - 冲突检测：本地/远程都有改动时弹窗让你选择保留哪边
  - 密码保存在 Windows 凭据管理器，不写入配置文件
- **托盘驻留**：关闭窗口缩到托盘、单实例、全局快捷键（默认 `Ctrl+Shift+D`）呼出/隐藏、可开机自启
- **低占用**：后台页面超时自动休眠回收（可配），内存随实际使用走
- **内网友好**：可忽略 HTTPS 自签证书错误（重启生效）；页面内 `window.open` 转系统浏览器
- 每服务独立缩放；favicon 自动识别；深色/浅色/跟随系统

## 界面

```
┌─────────────┬──────────────────────────────────────┐
│  搜索/过滤   │  ←  →  ⟳   http://nas:5000/...     ↗ │
│─────────────┼──────────────────────────────────────│
│ ▣ Portainer │                                      │
│ ▣ qBittorr… │          当前服务的页面                │
│ ▾ 监控       │       （独立 WebView2 渲染）           │
│   ▣ Grafana │                                      │
│─────────────┼──────────────────────────────────────│
│ ＋服务 ⊞ ☰ ⚙│                                      │
└─────────────┴──────────────────────────────────────┘
```

- 服务项右键：编辑 / 复制地址 / 在浏览器打开 / 关闭页面 / 删除
- 分组标题右键：重命名 / 添加服务到此组 / 删除分组
- 拖拽服务项排序，拖到分组标题可移动入组

## 开发

环境要求：Node 20+、Rust stable、Windows（WebView2，Win10/11 自带）。

```bash
npm install
npm run tauri dev      # 开发运行
npm run tauri build    # 打包（NSIS 安装包）
npx svelte-check       # 前端类型检查
```

> Linux 上可运行但多 webview 布局受 wry X11 实现限制（开发用）；**目标平台是 Windows**，
> Windows 下每个子 webview 是独立 HWND，定位/缩放精确。
>
> Rust 测试：`cd src-tauri && cargo test`（WebDAV 往返 / 认证 / rev 语义等 6 个用例）。

## 构建产物

`src-tauri/target/release/bundle/nsis/DockBox_0.1.0_x64-setup.exe`（当前用户安装，无需管理员）。
GitHub Actions（`.github/workflows/build.yml`）在 push 时自动构建并上传安装包。

## 配置与数据

| 文件 | 位置 | 内容 |
|---|---|---|
| `data.json` | `%APPDATA%\com.dockbox.app\` | 所有 Profile、服务、分组、主题、休眠设置（**参与同步**） |
| `settings.json` | 同上 | 托盘/自启/快捷键/证书开关/WebDAV 连接信息（不同步） |
| 凭据 | Windows 凭据管理器（`dockbox/webdav`） | WebDAV 密码 |
| 页面登录态 | `%APPDATA%\com.dockbox.app\svc-webview\` | 各服务的 Cookie / LocalStorage |

## WebDAV 同步说明

- 远程文件默认 `/dockbox/dockbox.json`，纯 JSON，可直接查看/备份
- 版本号 `rev` 单调递增：本地改动 `rev+1`；上传取 `max(本地, 远程)+1`
- 只同步「配置数据」，不同步本机偏好（托盘、自启、快捷键、WebDAV 地址等），多台机器各自设置一次即可
- 坚果云、Nextcloud、群晖、Nginx DAV 等任意支持 Basic 认证的 WebDAV 均可

## 已知限制

- 「忽略证书错误」是全局开关（WebView2 机制限制），重启后生效
- 弹窗（`window.open`）统一转到系统默认浏览器打开
- 目前仅提供 Windows 构建

## License

MIT
