<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let { saveBeforeUpdate }: { saveBeforeUpdate: () => Promise<void> } = $props();
  let dialog: HTMLDialogElement;
  let busy = $state(false);
  let installing = $state(false);
  let message = $state("");

  async function checkForUpdates() {
    if (busy) return;
    busy = true;
    message = "Checking GitHub for a new release…";
    dialog.showModal();
    try {
      const version = await invoke<string | null>("check_for_updates");
      if (!dialog.open) return;
      if (!version) {
        message = "You’re up to date.";
        return;
      }
      installing = true;
      message = `Saving open workbooks before updating to V${version}…`;
      await saveBeforeUpdate();
      message = `Downloading and installing V${version}…`;
      const result = await invoke<"installer-started" | "installed">("install_update", { version });
      if (result === "installer-started") {
        message = "The installer is open. Close this window to continue setup.";
        await getCurrentWindow().close().catch(() => {});
      } else {
        message = `V${version} is installed. Reopen Document Studio Spreadsheets to use it.`;
      }
    } catch (error) {
      const detail = error instanceof Error ? error.message : String(error);
      message = `The update could not be completed. ${detail}`;
    } finally {
      installing = false;
      busy = false;
    }
  }
</script>

<button type="button" onclick={checkForUpdates} disabled={busy} aria-haspopup="dialog">
  Check For Updates
</button>
<dialog bind:this={dialog} class="app-update" aria-labelledby="update-title"
  oncancel={event => { if (installing) event.preventDefault(); }}>
  <h2 id="update-title">Software update</h2>
  <p role="status">{message}</p>
  <button type="button" disabled={installing} onclick={() => dialog.close()}>
    {busy && !installing ? "Cancel" : "Close"}
  </button>
</dialog>

<style>
  .app-update {
    width: min(28rem, 94vw);
    padding: 1.5rem;
    border: 1px solid var(--line);
    border-radius: 12px;
    box-shadow: var(--shadow);
    background: var(--surface);
    color: var(--text);
  }
  .app-update::backdrop { background: #09060dc0; }
  .app-update h2 { margin: 0; }
  .app-update p { overflow-wrap: anywhere; }
  .app-update button { display: block; margin-left: auto; }
</style>
