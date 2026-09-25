<script lang="ts">
  import { closeModal, confirmAction, deleteService, openModal, saveService, serviceById, store } from "../state.svelte";
  import { favColor } from "./Sidebar.svelte";

  const modal = $derived(store.modal?.kind === "service" ? store.modal : null);
  const editing = $derived(modal?.serviceId ? serviceById(modal.serviceId) : null);

  let name = $state("");
  let url = $state("");
  let groupId = $state<string | null>(null);
  let icon = $state("");
  let zoom = $state(1);

  let inited = false;
  $effect(() => {
    const m = modal;
    if (!m || inited) return;
    inited = true;
    if (editing) {
      name = editing.name;
      url = editing.url;
      groupId = editing.groupId;
      icon = editing.icon ?? "";
      zoom = editing.zoom;
    } else {
      name = "";
      url = "";
      groupId = m.groupId ?? null;
      icon = "";
      zoom = 1;
    }
  });

  const groups = $derived(
    store.data ? Object.values(store.data.profiles[store.data.activeProfileId]?.groups ?? {}) : []
  );

  function normalizeUrl(u: string): string {
    const t = u.trim();
    if (!t) return t;
    if (/^[a-z][a-z0-9+.-]*:\/\//i.test(t)) return t;
    return "http://" + t;
  }

  function submit() {
    if (!name.trim() || !url.trim()) return;
    saveService({
      id: editing?.id,
      name: name.trim(),
      url: normalizeUrl(url),
      groupId,
      icon: icon.trim() || null,
      zoom,
    });
    closeModal();
  }
</script>

<div class="overlay" onclick={(e) => e.target === e.currentTarget && closeModal()}>
  <div class="modal">
    <div class="modal-head">
      <span>{editing ? "编辑服务" : "添加服务"}</span>
      <button class="btn icon" onclick={() => closeModal()}>✕</button>
    </div>
    <div class="modal-body">
      <div class="field">
        <label>名称</label>
        <input placeholder="如：Portainer" bind:value={name} />
      </div>
      <div class="field">
        <label>地址</label>
        <input placeholder="http://192.168.1.10:9000" bind:value={url} />
      </div>
      <div class="field">
        <label>分组</label>
        <select bind:value={groupId}>
          <option value={null}>未分组</option>
          {#each groups as g (g.id)}
            <option value={g.id}>{g.name}</option>
          {/each}
        </select>
      </div>
      <div class="field">
        <label>图标地址（可选，默认自动取站点图标）</label>
        <input placeholder="https://…/favicon.ico" bind:value={icon} />
      </div>
      <div class="field">
        <label>缩放：{Math.round(zoom * 100)}%</label>
        <input type="range" min="0.5" max="2" step="0.1" bind:value={zoom} />
      </div>
      {#if editing}
        <div class="row" style="margin-bottom:8px">
          <div class="fav" style="width:24px;height:24px;border-radius:6px;display:grid;place-items:center;color:#fff;background:{favColor(name || "?")}">
            {(name || "?").slice(0, 1).toUpperCase()}
          </div>
          <span class="hint">预览</span>
        </div>
        <button
          class="btn danger"
          onclick={() => {
            const id = editing!.id;
            closeModal();
            confirmAction({
              title: "删除服务",
              message: `确定删除「${name}」？`,
              confirmLabel: "删除",
              danger: true,
              onConfirm: () => deleteService(id),
            });
          }}>删除此服务</button
        >
      {/if}
    </div>
    <div class="modal-foot">
      <button class="btn" onclick={() => closeModal()}>取消</button>
      <button class="btn primary" onclick={submit}>{editing ? "保存" : "添加"}</button>
    </div>
  </div>
</div>
