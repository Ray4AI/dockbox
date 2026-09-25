<script lang="ts">
  import { closeModal, saveGroup, store } from "../state.svelte";

  const modal = $derived(store.modal?.kind === "group" ? store.modal : null);
  const isNew = $derived(!modal?.groupId);
  const groupName = $derived(
    modal?.groupId
      ? (store.data?.profiles[store.data.activeProfileId]?.groups.find((g) => g.id === modal.groupId)?.name ?? "")
      : ""
  );

  let name = $state("");
  let inited = false;
  $effect(() => {
    if (!modal || inited) return;
    inited = true;
    name = isNew ? "" : groupName;
  });

  function submit() {
    if (!name.trim()) return;
    saveGroup(isNew ? null : modal!.groupId, name.trim());
    closeModal();
  }
</script>

<div class="overlay" onclick={(e) => e.target === e.currentTarget && closeModal()}>
  <div class="modal" style="width:min(380px,calc(100vw - 48px))">
    <div class="modal-head">
      <span>{isNew ? "新建分组" : "重命名分组"}</span>
      <button class="btn icon" onclick={() => closeModal()}>✕</button>
    </div>
    <div class="modal-body">
      <div class="field">
        <label>分组名称</label>
        <input
          placeholder="如：监控"
          bind:value={name}
          onkeydown={(e) => e.key === "Enter" && submit()}
        />
      </div>
    </div>
    <div class="modal-foot">
      <button class="btn" onclick={() => closeModal()}>取消</button>
      <button class="btn primary" onclick={submit}>确定</button>
    </div>
  </div>
</div>
