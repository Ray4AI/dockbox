<script lang="ts">
  import {
    activateService,
    closeServicePage,
    confirmAction,
    deleteGroup,
    deleteService,
    moveServiceToGroup,
    openModal,
    reorderService,
    sidebarEntries,
    store,
    toggleGroup,
  } from "../state.svelte";
  import { openExternal } from "../api";

  let dragId = $state<string | null>(null);
  let dragOverId = $state<string | null>(null);
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  interface MenuItem {
    label: string;
    action?: () => void;
    danger?: boolean;
    sep?: boolean;
  }

  function showMenu(e: MouseEvent, items: MenuItem[]) {
    e.preventDefault();
    e.stopPropagation();
    menu = { x: e.clientX, y: e.clientY, items };
  }

  function closeMenu() {
    menu = null;
  }

  $effect(() => {
    if (!menu) return;
    const h = () => closeMenu();
    window.addEventListener("click", h);
    return () => window.removeEventListener("click", h);
  });

  function serviceMenu(e: MouseEvent, id: string, name: string, url: string) {
    showMenu(e, [
      { label: "编辑", action: () => openModal({ kind: "service", serviceId: id }) },
      { label: "复制地址", action: () => navigator.clipboard.writeText(url) },
      { label: "在浏览器打开", action: () => openExternal(url) },
      { label: "关闭页面", action: () => closeServicePage(id) },
      { label: "", sep: true },
      {
        label: "删除",
        danger: true,
        action: () =>
          confirmAction({
            title: "删除服务",
            message: `确定删除「${name}」？`,
            confirmLabel: "删除",
            danger: true,
            onConfirm: () => deleteService(id),
          }),
      },
    ]);
  }

  function groupMenu(e: MouseEvent, id: string, name: string) {
    showMenu(e, [
      { label: "重命名", action: () => openModal({ kind: "group", groupId: id }) },
      { label: "添加服务到此组", action: () => openModal({ kind: "service", groupId: id }) },
      { label: "", sep: true },
      {
        label: "删除分组",
        danger: true,
        action: () =>
          confirmAction({
            title: "删除分组",
            message: `删除「${name}」？组内服务将变为未分组。`,
            confirmLabel: "删除",
            danger: true,
            onConfirm: () => deleteGroup(id),
          }),
      },
    ]);
  }

  const entries = $derived(sidebarEntries());
  const profileName = $derived(
    store.data ? (store.data.profiles[store.data.activeProfileId]?.name ?? "") : ""
  );
</script>

<aside class="sidebar" style="--sw:{store.sidebarWidth}px">
  <div class="sidebar-head">
    <div class="brand">
      <div class="logo">D</div>
      <span>DockBox</span>
      <span class="badge" style="margin-left:auto">{profileName}</span>
    </div>
    <input class="search" placeholder="搜索服务…" bind:value={store.query} />
  </div>

  <div class="list">
    {#each entries as entry (entry.group?.id ?? "ungrouped")}
      {#if entry.group}
        <div
          class="group-header"
          onclick={() => toggleGroup(entry.group!.id)}
          oncontextmenu={(e) => groupMenu(e, entry.group!.id, entry.group!.name)}
          ondragover={(e) => e.preventDefault()}
          ondrop={(e) => {
            e.preventDefault();
            if (dragId) moveServiceToGroup(dragId, entry.group!.id);
            dragId = null;
            dragOverId = null;
          }}
        >
          <span class="chev">{entry.group.collapsed ? "▸" : "▾"}</span>
          <span>{entry.group.name}</span>
          <span class="count">{entry.services.length}</span>
        </div>
      {/if}

      {#if !entry.group?.collapsed}
        {#each entry.services as svc (svc.id)}
          <div
            class="item"
            class:active={store.activeServiceId === svc.id}
            class:drag-over={dragOverId === svc.id}
            draggable="true"
            onclick={() => activateService(svc.id)}
            oncontextmenu={(e) => serviceMenu(e, svc.id, svc.name, svc.url)}
            ondragstart={(e) => {
              dragId = svc.id;
              if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
            }}
            ondragover={(e) => {
              e.preventDefault();
              dragOverId = svc.id;
            }}
            ondragleave={() => (dragOverId = null)}
            ondrop={(e) => {
              e.preventDefault();
              e.stopPropagation();
              if (dragId && dragId !== svc.id) reorderService(dragId, svc.id, svc.groupId);
              dragId = null;
              dragOverId = null;
            }}
          >
            <div class="fav" style="background:{favColor(svc.name)}">
              {#if svc.icon}
                <img src={svc.icon} alt="" />
              {:else}
                {svc.name.slice(0, 1).toUpperCase()}
              {/if}
            </div>
            <div class="name" title={svc.url}>{svc.name}</div>
            {#if store.lastActive[svc.id] && store.activeServiceId === svc.id}
              <div class="dot"></div>
            {/if}
          </div>
        {/each}
      {/if}
    {/each}

    {#if entries.every((e) => e.services.length === 0)}
      <div class="empty-hint">
        还没有服务<br />
        点击下方「＋」添加你的第一个 Docker 工具
      </div>
    {/if}
  </div>

  <div class="sidebar-foot">
    <button class="btn" style="flex:1" onclick={() => openModal({ kind: "service" })}>＋ 服务</button>
    <button class="btn icon" title="新建分组" onclick={() => openModal({ kind: "group", groupId: "" })}>⊞</button>
    <button class="btn icon" title="配置管理" onclick={() => openModal({ kind: "profile" })}>☰</button>
    <button class="btn icon" title="设置" onclick={() => openModal({ kind: "settings", tab: "general" })}>⚙</button>
  </div>
</aside>

{#if menu}
  <div class="ctx" style="left:{menu.x}px;top:{menu.y}px">
    {#each menu.items as it}
      {#if it.sep}
        <div class="sep"></div>
      {:else}
        <div
          class="ctx-item"
          class:danger={it.danger}
          onclick={() => {
            it.action?.();
            closeMenu();
          }}
        >
          {it.label}
        </div>
      {/if}
    {/each}
  </div>
{/if}

<script module lang="ts">
  const palette = ["#4f8cff", "#22c55e", "#f59e0b", "#ef4444", "#a855f7", "#14b8a6", "#ec4899"];
  export function favColor(name: string): string {
    let h = 0;
    for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
    return palette[h % palette.length];
  }
</script>
