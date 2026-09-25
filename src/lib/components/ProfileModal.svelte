<script lang="ts">
  import {
    closeModal,
    confirmAction,
    createProfile,
    deleteProfile,
    duplicateProfile,
    renameProfile,
    store,
    switchProfile,
  } from "../state.svelte";

  let newName = $state("");
  let editingId = $state<string | null>(null);
  let editingName = $state("");

  const profiles = $derived(
    store.data ? Object.values(store.data.profiles).sort((a, b) => a.updatedAt - b.updatedAt) : []
  );

  function addProfile() {
    if (!newName.trim()) return;
    createProfile(newName.trim());
    newName = "";
  }

  function commitRename(id: string) {
    if (editingName.trim()) renameProfile(id, editingName.trim());
    editingId = null;
  }
</script>

<div class="overlay" onclick={(e) => e.target === e.currentTarget && closeModal()}>
  <div class="modal">
    <div class="modal-head">
      <span>配置管理</span>
      <button class="btn icon" onclick={() => closeModal()}>✕</button>
    </div>
    <div class="modal-body">
      <div class="hint" style="margin-bottom:10px">
        每个配置包含独立的服务列表与分组，适合不同环境（家里 / 公司 / 实验室）。WebDAV 同步会同步所有配置。
      </div>

      {#each profiles as p (p.id)}
        <div
          class="row between"
          style="padding:8px 10px;border:1px solid var(--border);border-radius:8px;margin-bottom:6px"
        >
          {#if editingId === p.id}
            <input
              class="search"
              style="flex:1"
              bind:value={editingName}
              onkeydown={(e) => e.key === "Enter" && commitRename(p.id)}
            />
            <button class="btn" onclick={() => commitRename(p.id)}>保存</button>
          {:else}
            <div class="row" style="gap:8px;flex:1;min-width:0">
              <span
                style="font-weight:600;cursor:pointer"
                onclick={() => {
                  switchProfile(p.id);
                  closeModal();
                }}>{p.name}</span
              >
              {#if p.id === store.data?.activeProfileId}
                <span class="badge ok">当前</span>
              {/if}
              <span class="hint">{p.services.length} 个服务</span>
            </div>
            <button
              class="btn icon"
              title="重命名"
              onclick={() => {
                editingId = p.id;
                editingName = p.name;
              }}>✎</button
            >
            <button class="btn icon" title="复制" onclick={() => duplicateProfile(p.id)}>⧉</button>
            <button
              class="btn icon"
              title="删除"
              onclick={() =>
                confirmAction({
                  title: "删除配置",
                  message: `确定删除配置「${p.name}」及其所有服务？`,
                  confirmLabel: "删除",
                  danger: true,
                  onConfirm: () => deleteProfile(p.id),
                })}>🗑</button
            >
          {/if}
        </div>
      {/each}

      <div class="row" style="margin-top:12px">
        <input class="search" placeholder="新配置名称，如：公司 NAS" bind:value={newName} />
        <button class="btn primary" onclick={addProfile}>新建</button>
      </div>
    </div>
    <div class="modal-foot">
      <button class="btn" onclick={() => closeModal()}>关闭</button>
    </div>
  </div>
</div>
