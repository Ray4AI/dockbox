<script lang="ts">
  import { openExternal } from "../api";
  import * as api from "../api";
  import { saveService, serviceById, store, toast } from "../state.svelte";

  const svc = $derived(store.activeServiceId ? serviceById(store.activeServiceId) : null);

  function nav(action: "back" | "forward" | "reload") {
    if (!store.activeServiceId) return;
    api.webviewNav(store.activeServiceId, action).catch((e) => toast(`操作失败：${e}`));
  }

  function zoomBy(delta: number) {
    if (!svc || !store.activeServiceId) return;
    const z = Math.min(2, Math.max(0.5, Math.round((svc.zoom + delta) * 10) / 10));
    saveService({ ...svc, zoom: z });
  }
</script>

<div class="toolbar">
  <button class="btn icon" title="后退" disabled={!svc} onclick={() => nav("back")}>←</button>
  <button class="btn icon" title="前进" disabled={!svc} onclick={() => nav("forward")}>→</button>
  <button class="btn icon" title="刷新" disabled={!svc} onclick={() => nav("reload")}>⟳</button>
  <div class="title" title={svc?.name}>{svc?.name ?? ""}</div>
  <div class="url mono" title={svc?.url}>{svc?.url ?? "未选择服务"}</div>
  {#if svc}
    <button class="btn icon" title="缩小" onclick={() => zoomBy(-0.1)}>－</button>
    <span class="badge">{Math.round(svc.zoom * 100)}%</span>
    <button class="btn icon" title="放大" onclick={() => zoomBy(0.1)}>＋</button>
  {/if}
  <button class="btn icon" title="在系统浏览器打开" disabled={!svc} onclick={() => svc && openExternal(svc.url)}>↗</button>
</div>
