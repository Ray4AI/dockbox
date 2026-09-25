<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toolbar from "./lib/components/Toolbar.svelte";
  import ServiceForm from "./lib/components/ServiceForm.svelte";
  import SettingsModal from "./lib/components/SettingsModal.svelte";
  import ProfileModal from "./lib/components/ProfileModal.svelte";
  import GroupForm from "./lib/components/GroupForm.svelte";
  import ConflictModal from "./lib/components/ConflictModal.svelte";
  import ConfirmModal from "./lib/components/ConfirmModal.svelte";
  import { closeModal, init, layout, openModal, setSidebarWidth, store } from "./lib/state.svelte";

  let resizing = $state(false);

  onMount(() => {
    init();
    const win = getCurrentWindow();
    const unlisten = win.onResized(() => {
      layout();
    });
    const onResize = () => layout();
    window.addEventListener("resize", onResize);
    return () => {
      unlisten.then((f) => f());
      window.removeEventListener("resize", onResize);
    };
  });

  function startResize(e: MouseEvent) {
    e.preventDefault();
    resizing = true;
    const startX = e.clientX;
    const startW = store.sidebarWidth;
    const move = (ev: MouseEvent) => setSidebarWidth(startW + ev.clientX - startX);
    const up = () => {
      resizing = false;
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && store.modal) closeModal();
  }}
/>

<div class="app" class:resizing>
  <Sidebar />
  <div class="resizer" onmousedown={startResize}></div>
  <div class="main">
    <Toolbar />
    <div class="content">
      {#if !store.activeServiceId}
        <div class="welcome">
          <div class="logo-big">D</div>
          <h1>DockBox</h1>
          <div>你的 Docker 小工具面板</div>
          <div class="hint">
            在左侧选择一个服务开始使用，或点击下方添加。<br />
            所有页面都在本地渲染，关闭窗口后驻留托盘继续待命。
          </div>
          <div class="row" style="margin-top:8px">
            <button class="btn primary" onclick={() => openModal({ kind: "service" })}>＋ 添加服务</button>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

{#if store.toast}
  <div class="toast">{store.toast}</div>
{/if}

{#if store.modal}
  {#if store.modal.kind === "service"}
    <ServiceForm />
  {:else if store.modal.kind === "settings"}
    <SettingsModal />
  {:else if store.modal.kind === "profile"}
    <ProfileModal />
  {:else if store.modal.kind === "group"}
    <GroupForm />
  {:else if store.modal.kind === "conflict"}
    <ConflictModal />
  {:else if store.modal.kind === "confirm"}
    <ConfirmModal />
  {/if}
{/if}
