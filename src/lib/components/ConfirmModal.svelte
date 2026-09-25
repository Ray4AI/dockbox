<script lang="ts">
  import { closeModal, store } from "../state.svelte";

  const modal = $derived(store.modal?.kind === "confirm" ? store.modal : null);
</script>

<div class="overlay" onclick={(e) => e.target === e.currentTarget && closeModal()}>
  <div class="modal" style="width:min(400px,calc(100vw - 48px))">
    <div class="modal-head"><span>{modal?.title}</span></div>
    <div class="modal-body">
      <div class="hint" style="line-height:1.8">{modal?.message}</div>
    </div>
    <div class="modal-foot">
      <button class="btn" onclick={() => closeModal()}>取消</button>
      <button
        class="btn {modal?.danger ? 'danger' : 'primary'}"
        onclick={() => {
          const fn = modal?.onConfirm;
          closeModal();
          fn?.();
        }}>{modal?.confirmLabel ?? "确定"}</button
      >
    </div>
  </div>
</div>
