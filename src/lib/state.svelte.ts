import {
  getAppState,
  saveData,
  saveSettings as apiSaveSettings,
  webviewActivate,
  webviewClose,
  webviewCloseAll,
  webviewHideAll,
  webviewOpen,
  webviewSetBounds,
  webviewSetZoom,
  webviewShow,
  webdavPull,
  webdavPush,
} from "./api";
import type {
  AppData,
  Bounds,
  Group,
  LocalSettings,
  Profile,
  Service,
  SyncOutcome,
} from "./types";

export const TOOLBAR_H = 44;

export type Modal =
  | { kind: "service"; serviceId?: string; groupId?: string | null }
  | { kind: "group"; groupId: string }
  | { kind: "settings"; tab: "general" | "webdav" | "data" }
  | { kind: "profile" }
  | { kind: "conflict"; action: "push" | "pull"; outcome: SyncOutcome }
  | {
      kind: "confirm";
      title: string;
      message: string;
      confirmLabel?: string;
      danger?: boolean;
      onConfirm: () => void;
    };

function newId(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

export const store = $state({
  loaded: false,
  data: null as AppData | null,
  settings: null as LocalSettings | null,
  hasWebdavPassword: false,
  activeServiceId: null as string | null,
  query: "",
  sidebarWidth: 240,
  modal: null as Modal | null,
  toast: null as string | null,
  syncing: false,
  /** serviceId -> 最后一次激活的时间戳（用于休眠回收） */
  lastActive: {} as Record<string, number>,
});

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let syncTimer: ReturnType<typeof setTimeout> | null = null;
let toastTimer: ReturnType<typeof setTimeout> | null = null;

/* ---------------- 初始化 ---------------- */

export async function init(): Promise<void> {
  const w = localStorage.getItem("dockbox.sidebarWidth");
  if (w) store.sidebarWidth = Math.max(180, Math.min(480, parseInt(w, 10) || 240));

  const st = await getAppState();
  store.data = st.data;
  store.settings = st.settings;
  store.hasWebdavPassword = st.hasWebdavPassword;
  store.loaded = true;
  applyTheme();

  // 启动自动拉取
  if (st.settings.webdav.autoSync && st.settings.webdav.url) {
    setTimeout(() => {
      syncPull(false).catch(() => {});
    }, 800);
  }

  // 休眠回收巡检
  setInterval(sweepIdle, 20_000);
}

export function applyTheme(): void {
  const theme = store.data?.prefs.theme ?? "system";
  const el = document.documentElement;
  if (theme === "system") {
    el.removeAttribute("data-theme");
  } else {
    el.setAttribute("data-theme", theme);
  }
}

/* ---------------- 派生数据 ---------------- */

export function activeProfile(): Profile | null {
  if (!store.data) return null;
  return store.data.profiles[store.data.activeProfileId] ?? null;
}

export interface SidebarEntry {
  group: Group | null;
  services: Service[];
}

export function sidebarEntries(): SidebarEntry[] {
  const p = activeProfile();
  if (!p) return [];
  const q = store.query.trim().toLowerCase();
  const match = (s: Service) =>
    !q || s.name.toLowerCase().includes(q) || s.url.toLowerCase().includes(q);
  const groups = [...p.groups].sort((a, b) => a.order - b.order);
  const ungrouped = p.services
    .filter((s) => !s.groupId && match(s))
    .sort((a, b) => a.order - b.order);
  const entries: SidebarEntry[] = [];
  for (const g of groups) {
    const services = p.services
      .filter((s) => s.groupId === g.id && match(s))
      .sort((a, b) => a.order - b.order);
    if (services.length > 0 || !q) entries.push({ group: g, services });
  }
  if (ungrouped.length > 0 || (!q && entries.length === 0)) {
    entries.push({ group: null, services: ungrouped });
  }
  return entries;
}

export function serviceById(id: string): Service | null {
  return activeProfile()?.services.find((s) => s.id === id) ?? null;
}

/* ---------------- 数据持久化 ---------------- */

function touchProfile(): void {
  const p = activeProfile();
  if (p) p.updatedAt = Date.now();
}

export function persist(): void {
  if (!store.data) return;
  touchProfile();
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    if (!store.data) return;
    try {
      const rev = await saveData(store.data);
      store.data.rev = rev;
      scheduleSync();
    } catch (e) {
      toast(`保存失败：${e}`);
    }
  }, 300);
}

function scheduleSync(): void {
  const s = store.settings;
  if (!s?.webdav.autoSync || !s.webdav.url) return;
  if (syncTimer) clearTimeout(syncTimer);
  syncTimer = setTimeout(() => {
    syncPush(false).catch(() => {});
  }, 3000);
}

export async function saveSettings(s: LocalSettings): Promise<void> {
  await apiSaveSettings(s);
  store.settings = JSON.parse(JSON.stringify(s));
  applyTheme();
}

/* ---------------- Toast / Modal ---------------- */

export function toast(msg: string): void {
  store.toast = msg;
  if (toastTimer) clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (store.toast = null), 3200);
}

export function openModal(m: Modal): void {
  store.modal = m;
  if (store.activeServiceId) webviewHideAll().catch(() => {});
}

export function closeModal(): void {
  store.modal = null;
  if (store.activeServiceId && serviceById(store.activeServiceId)) {
    webviewShow(store.activeServiceId, computeBounds()).catch(() => {});
  }
}

export function confirmAction(opts: {
  title: string;
  message: string;
  confirmLabel?: string;
  danger?: boolean;
  onConfirm: () => void;
}): void {
  openModal({ kind: "confirm", ...opts });
}

/* ---------------- 布局 ---------------- */

export function computeBounds(): Bounds {
  return {
    x: store.sidebarWidth,
    y: TOOLBAR_H,
    width: Math.max(100, window.innerWidth - store.sidebarWidth),
    height: Math.max(100, window.innerHeight - TOOLBAR_H),
  };
}

export function setSidebarWidth(w: number): void {
  store.sidebarWidth = Math.max(180, Math.min(480, Math.round(w)));
  localStorage.setItem("dockbox.sidebarWidth", String(store.sidebarWidth));
  if (store.activeServiceId && !store.modal) {
    webviewSetBounds(store.activeServiceId, computeBounds()).catch(() => {});
  }
}

export async function layout(): Promise<void> {
  if (store.activeServiceId && !store.modal) {
    await webviewSetBounds(store.activeServiceId, computeBounds()).catch(() => {});
  }
}

/* ---------------- 服务 / 分组 操作 ---------------- */

export async function activateService(id: string): Promise<void> {
  const svc = serviceById(id);
  if (!svc) return;
  store.activeServiceId = id;
  store.lastActive[id] = Date.now();
  try {
    await webviewOpen(id, svc.url, svc.zoom);
    if (store.modal) {
      await webviewHideAll();
    } else {
      await webviewActivate(id, computeBounds());
    }
  } catch (e) {
    toast(`打开页面失败：${e}`);
  }
}

export function closeServicePage(id: string): void {
  if (store.activeServiceId === id) store.activeServiceId = null;
  webviewClose(id).catch(() => {});
  delete store.lastActive[id];
}

export function saveService(input: {
  id?: string;
  name: string;
  url: string;
  groupId: string | null;
  icon: string | null;
  zoom: number;
}): void {
  const p = activeProfile();
  if (!p) return;
  if (input.id) {
    const svc = p.services.find((s) => s.id === input.id);
    if (svc) {
      const urlChanged = svc.url !== input.url;
      Object.assign(svc, {
        name: input.name,
        url: input.url,
        groupId: input.groupId,
        icon: input.icon,
        zoom: input.zoom,
      });
      if (urlChanged) {
        webviewClose(svc.id).catch(() => {});
        delete store.lastActive[svc.id];
        if (store.activeServiceId === svc.id) store.activeServiceId = null;
      } else if (store.activeServiceId === svc.id) {
        webviewSetZoom(svc.id, input.zoom).catch(() => {});
      }
    }
  } else {
    const maxOrder = Math.max(0, ...p.services.map((s) => s.order));
    p.services.push({
      id: newId(),
      name: input.name,
      url: input.url,
      groupId: input.groupId,
      icon: input.icon,
      zoom: input.zoom,
      order: maxOrder + 1,
    });
  }
  persist();
}

export function deleteService(id: string): void {
  const p = activeProfile();
  if (!p) return;
  p.services = p.services.filter((s) => s.id !== id);
  closeServicePage(id);
  persist();
}

export function saveGroup(id: string | null, name: string): void {
  const p = activeProfile();
  if (!p) return;
  if (id) {
    const g = p.groups.find((x) => x.id === id);
    if (g) g.name = name;
  } else {
    const maxOrder = Math.max(0, ...p.groups.map((g) => g.order));
    p.groups.push({ id: newId(), name, order: maxOrder + 1, collapsed: false });
  }
  persist();
}

export function deleteGroup(id: string): void {
  const p = activeProfile();
  if (!p) return;
  for (const s of p.services) if (s.groupId === id) s.groupId = null;
  p.groups = p.groups.filter((g) => g.id !== id);
  persist();
}

export function toggleGroup(id: string): void {
  const p = activeProfile();
  const g = p?.groups.find((x) => x.id === id);
  if (g) {
    g.collapsed = !g.collapsed;
    persist();
  }
}

/** 拖拽排序：把 dragId 移到 targetId 之前（同一分组内） */
export function reorderService(dragId: string, targetId: string, groupId: string | null): void {
  const p = activeProfile();
  if (!p || dragId === targetId) return;
  const list = p.services
    .filter((s) => s.groupId === groupId)
    .sort((a, b) => a.order - b.order);
  const from = list.findIndex((s) => s.id === dragId);
  const to = list.findIndex((s) => s.id === targetId);
  if (from < 0 || to < 0) return;
  const [moved] = list.splice(from, 1);
  moved.groupId = groupId;
  list.splice(to, 0, moved);
  list.forEach((s, i) => (s.order = i + 1));
  persist();
}

export function moveServiceToGroup(serviceId: string, groupId: string | null): void {
  const p = activeProfile();
  if (!p) return;
  const s = p.services.find((x) => x.id === serviceId);
  if (!s || s.groupId === groupId) return;
  s.groupId = groupId;
  const list = p.services.filter((x) => x.groupId === groupId);
  s.order = Math.max(0, ...list.map((x) => x.order)) + 1;
  persist();
}

/* ---------------- Profile 操作 ---------------- */

export function switchProfile(id: string): void {
  if (!store.data || !store.data.profiles[id]) return;
  store.data.activeProfileId = id;
  store.activeServiceId = null;
  webviewCloseAll().catch(() => {});
  persist();
}

export function createProfile(name: string): void {
  if (!store.data) return;
  const id = newId();
  store.data.profiles[id] = {
    id,
    name,
    updatedAt: Date.now(),
    services: [],
    groups: [],
  };
  switchProfile(id);
}

export function duplicateProfile(id: string): void {
  if (!store.data) return;
  const src = store.data.profiles[id];
  if (!src) return;
  const copy: Profile = JSON.parse(JSON.stringify(src));
  copy.id = newId();
  copy.name = `${src.name} 副本`;
  copy.updatedAt = Date.now();
  // 重新生成 id，避免交叉引用
  const groupMap = new Map<string, string>();
  for (const g of copy.groups) {
    const nid = newId();
    groupMap.set(g.id, nid);
    g.id = nid;
  }
  for (const s of copy.services) {
    s.id = newId();
    s.groupId = s.groupId ? (groupMap.get(s.groupId) ?? null) : null;
  }
  store.data.profiles[copy.id] = copy;
  persist();
}

export function renameProfile(id: string, name: string): void {
  const p = store.data?.profiles[id];
  if (p) {
    p.name = name;
    persist();
  }
}

export function deleteProfile(id: string): void {
  if (!store.data) return;
  const keys = Object.keys(store.data.profiles);
  if (keys.length <= 1) {
    toast("至少保留一个配置");
    return;
  }
  delete store.data.profiles[id];
  if (store.data.activeProfileId === id) {
    store.data.activeProfileId = Object.keys(store.data.profiles)[0];
    store.activeServiceId = null;
    webviewCloseAll().catch(() => {});
  }
  persist();
}

/* ---------------- 同步 ---------------- */

export async function syncPush(force: boolean): Promise<SyncOutcome> {
  store.syncing = true;
  try {
    const out = await webdavPush(force);
    handleSyncOutcome(out, "push");
    return out;
  } finally {
    store.syncing = false;
  }
}

export async function syncPull(force: boolean): Promise<SyncOutcome> {
  store.syncing = true;
  try {
    const out = await webdavPull(force);
    if (out.status === "pulled") {
      const st = await getAppState();
      store.data = st.data;
      store.settings = st.settings;
      store.activeServiceId = null;
      await webviewCloseAll().catch(() => {});
      toast("已从 WebDAV 拉取最新配置");
    } else {
      handleSyncOutcome(out, "pull");
    }
    return out;
  } finally {
    store.syncing = false;
  }
}

function handleSyncOutcome(out: SyncOutcome, action: "push" | "pull"): void {
  switch (out.status) {
    case "pushed":
      toast("已上传到 WebDAV");
      break;
    case "up-to-date":
      toast("配置已是最新");
      break;
    case "local-newer":
      toast("本地较新，已跳过");
      break;
    case "pulled":
      break;
    case "conflict":
      openModal({ kind: "conflict", action, outcome: out });
      break;
    default:
      if (out.message) toast(out.message);
  }
}

/* ---------------- 休眠回收 ---------------- */

function sweepIdle(): void {
  const minutes = store.data?.prefs.suspendMinutes ?? 0;
  if (minutes <= 0) return;
  const now = Date.now();
  for (const [id, ts] of Object.entries(store.lastActive)) {
    if (id === store.activeServiceId) continue;
    if (now - ts > minutes * 60_000) {
      webviewClose(id).catch(() => {});
      delete store.lastActive[id];
    }
  }
}
