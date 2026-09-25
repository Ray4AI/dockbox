<script lang="ts">
  import { closeModal, saveSettings, store, syncPull, syncPush, toast, confirmAction, persist } from "../state.svelte";
  import { webdavSetPassword, webdavTest } from "../api";
  import type { LocalSettings, Prefs } from "../types";

  const modal = $derived(store.modal?.kind === "settings" ? store.modal : null);
  let tab = $state<"general" | "webdav" | "data">("general");
  let draft = $state<LocalSettings | null>(null);
  let prefs = $state<Prefs | null>(null);
  let password = $state("");
  let testMsg = $state<{ ok: boolean; text: string } | null>(null);
  let exportText = $state("");
  let importText = $state("");

  let inited = false;
  $effect(() => {
    if (!modal || inited) return;
    inited = true;
    tab = modal.tab;
    draft = JSON.parse(JSON.stringify(store.settings));
    prefs = JSON.parse(JSON.stringify(store.data?.prefs ?? { theme: "system", suspendMinutes: 10 }));
  });

  async function save() {
    if (!draft || !prefs) return;
    if (password.trim()) {
      try {
        await webdavSetPassword(password.trim());
        store.hasWebdavPassword = true;
        password = "";
      } catch (e) {
        toast(`保存密码失败：${e}`);
        return;
      }
    }
    try {
      await saveSettings(draft);
      if (store.data) {
        store.data.prefs = prefs;
        persist();
      }
      toast("设置已保存");
      closeModal();
    } catch (e) {
      toast(`保存失败：${e}`);
    }
  }

  async function test() {
    if (!draft) return;
    testMsg = null;
    try {
      await saveSettings(draft);
      if (password.trim()) {
        await webdavSetPassword(password.trim());
        store.hasWebdavPassword = true;
      }
      const r = await webdavTest();
      testMsg = { ok: true, text: r };
    } catch (e) {
      testMsg = { ok: false, text: String(e) };
    }
  }

  function doExport() {
    exportText = JSON.stringify(store.data, null, 2);
  }

  async function doImport() {
    try {
      const parsed = JSON.parse(importText);
      if (!parsed.profiles || typeof parsed.profiles !== "object") throw new Error("格式不正确");
      await confirmAction({
        title: "导入配置",
        message: "导入将覆盖当前全部配置（含所有 Profile），确定？",
        confirmLabel: "导入",
        onConfirm: async () => {
          const { saveData } = await import("../api");
          if (!store.data) return;
          store.data = {
            version: parsed.version ?? 1,
            rev: store.data.rev,
            activeProfileId:
              parsed.activeProfileId && parsed.profiles[parsed.activeProfileId]
                ? parsed.activeProfileId
                : Object.keys(parsed.profiles)[0],
            profiles: parsed.profiles,
            prefs: parsed.prefs ?? store.data.prefs,
          };
          try {
            const rev = await saveData(store.data);
            store.data.rev = rev;
            const { webviewCloseAll } = await import("../api");
            await webviewCloseAll();
            store.activeServiceId = null;
            toast("导入成功");
            closeModal();
          } catch (e) {
            toast(`导入失败：${e}`);
          }
        },
      });
    } catch (e) {
      toast(`解析失败：${e}`);
    }
  }
</script>

<div class="overlay" onclick={(e) => e.target === e.currentTarget && closeModal()}>
  <div class="modal wide">
    <div class="modal-head">
      <span>设置</span>
      <button class="btn icon" onclick={() => closeModal()}>✕</button>
    </div>

    <div class="tabs">
      <div class="tab" class:active={tab === "general"} onclick={() => (tab = "general")}>通用</div>
      <div class="tab" class:active={tab === "webdav"} onclick={() => (tab = "webdav")}>WebDAV 同步</div>
      <div class="tab" class:active={tab === "data"} onclick={() => (tab = "data")}>导入 / 导出</div>
    </div>

    {#if draft && prefs}
      <div class="modal-body">
        {#if tab === "general"}
          <div class="field">
            <label>点击关闭按钮时</label>
            <select bind:value={draft.closeToTray}>
              <option value="tray">驻留托盘（推荐）</option>
              <option value="minimize">最小化到任务栏</option>
              <option value="quit">退出应用</option>
            </select>
          </div>
          <label class="check">
            <input type="checkbox" bind:checked={draft.startMinimized} />
            启动时最小化到托盘
          </label>
          <label class="check">
            <input type="checkbox" bind:checked={draft.autostart} />
            开机自动启动
          </label>
          <div class="field" style="margin-top:10px">
            <label>全局快捷键（呼出 / 隐藏窗口）</label>
            <input placeholder="Ctrl+Shift+D" bind:value={draft.globalShortcut} />
          </div>
          <label class="check">
            <input type="checkbox" bind:checked={draft.ignoreCertErrors} />
            忽略 HTTPS 证书错误（内网自签证书）<span class="badge warn">重启后生效</span>
          </label>
          <div class="field" style="margin-top:10px">
            <label>后台页面休眠回收（分钟，0 = 不回收）</label>
            <input type="number" min="0" max="120" bind:value={prefs.suspendMinutes} />
          </div>
          <div class="field">
            <label>主题</label>
            <select bind:value={prefs.theme}>
              <option value="system">跟随系统</option>
              <option value="dark">深色</option>
              <option value="light">浅色</option>
            </select>
          </div>
        {:else if tab === "webdav"}
          <div class="field">
            <label>服务器地址</label>
            <input placeholder="https://dav.example.com/dav" bind:value={draft.webdav.url} />
          </div>
          <div class="field">
            <label>用户名</label>
            <input placeholder="username" bind:value={draft.webdav.username} />
          </div>
          <div class="field">
            <label>应用密码 {store.hasWebdavPassword ? "（已保存，留空保持不变）" : ""}</label>
            <input type="password" placeholder={store.hasWebdavPassword ? "••••••••" : "应用专用密码"} bind:value={password} />
          </div>
          <div class="field">
            <label>远程路径</label>
            <input placeholder="/dockbox/dockbox.json" bind:value={draft.webdav.remotePath} />
          </div>
          <label class="check">
            <input type="checkbox" bind:checked={draft.webdav.autoSync} />
            自动同步（改动后上传、启动时拉取）
          </label>
          <div class="row" style="margin-top:10px;flex-wrap:wrap">
            <button class="btn" onclick={test}>测试连接</button>
            <button
              class="btn"
              onclick={() =>
                confirmAction({
                  title: "上传配置",
                  message: "将本地配置上传并覆盖远程文件，确定？",
                  confirmLabel: "上传",
                  onConfirm: () => syncPush(true),
                })}>上传（覆盖远程）</button
            >
            <button
              class="btn"
              onclick={() =>
                confirmAction({
                  title: "下载配置",
                  message: "将用远程配置覆盖本地（含所有 Profile），确定？",
                  confirmLabel: "下载",
                  onConfirm: () => syncPull(true),
                })}>下载（覆盖本地）</button
            >
            <button
              class="btn"
              disabled={!store.hasWebdavPassword && !password.trim()}
              onclick={() => {
                store.hasWebdavPassword = false;
                password = "";
                toast("已清除密码（保存设置后生效）");
              }}>清除密码</button
            >
          </div>
          {#if testMsg}
            <div class="hint" style="margin-top:10px;color:{testMsg.ok ? '#4ade80' : '#f87171'}">
              {testMsg.text}
            </div>
          {/if}
          <div class="hint" style="margin-top:12px">
            密码保存在 Windows 凭据管理器，不写入配置文件。同步内容为所有配置（Profile）与主题、休眠等偏好；
            托盘、自启、快捷键等本机设置不同步。
          </div>
        {:else}
          <div class="field">
            <label>导出（复制保存即可）</label>
            <textarea readonly placeholder="点击「生成导出内容」" bind:value={exportText}></textarea>
          </div>
          <button class="btn" onclick={doExport}>生成导出内容</button>
          <div class="field" style="margin-top:16px">
            <label>导入（粘贴 JSON，覆盖当前全部配置）</label>
            <textarea placeholder="粘贴之前导出的 JSON" bind:value={importText}></textarea>
          </div>
          <button class="btn" onclick={doImport} disabled={!importText.trim()}>导入</button>
        {/if}
      </div>

      <div class="modal-foot">
        <button class="btn" onclick={() => closeModal()}>取消</button>
        <button class="btn primary" onclick={save}>保存设置</button>
      </div>
    {/if}
  </div>
</div>
