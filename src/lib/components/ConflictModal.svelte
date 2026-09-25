<script lang="ts">
  import { closeModal, store, syncPull, syncPush } from "../state.svelte";

  const modal = $derived(store.modal?.kind === "conflict" ? store.modal : null);
</script>

<div class="overlay">
  <div class="modal" style="width:min(440px,calc(100vw - 48px))">
    <div class="modal-head"><span>同步冲突</span></div>
    <div class="modal-body">
      <div class="hint" style="line-height:1.9">
        本地和远程配置都有新的修改，需要选择保留哪一边：<br />
        本地版本号 <b class="mono">v{modal?.outcome.localRev}</b> ·
        远程版本号 <b class="mono">v{modal?.outcome.remoteRev}</b>
        <br /><br />
        · <b>用本地覆盖远程</b>：当前这台机器的配置胜出<br />
        · <b>用远程覆盖本地</b>：另一台机器的配置胜出（本地改动丢失）
      </div>
    </div>
    <div class="modal-foot">
      <button class="btn" onclick={() => closeModal()}>稍后处理</button>
      <button
        class="btn"
        onclick={async () => {
          closeModal();
          await syncPull(true);
        }}>用远程覆盖本地</button
      >
      <button
        class="btn primary"
        onclick={async () => {
          closeModal();
          await syncPush(true);
        }}>用本地覆盖远程</button
      >
    </div>
  </div>
</div>
