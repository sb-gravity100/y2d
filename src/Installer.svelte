<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";

  let destination = $state("");
  let version = $state("");
  let shortcuts = $state(true);
  let busy = $state(false);
  let done = $state(false);
  let error = $state("");

  onMount(() => {
    console.info("[y2d-setup] loading setup preferences");
    void Promise.all([invoke<string>("setup_destination"), getVersion()])
      .then(([path, current]) => { destination = path; version = current; })
      .catch((reason) => { console.error("[y2d-setup] setup initialization failed", reason); error = String(reason); });
  });

  async function chooseFolder() {
    console.info("[y2d-setup] choosing installation location");
    try {
      const path = await open({ directory: true, multiple: false, title: "Choose an empty folder for y2d" });
      if (typeof path === "string") destination = path;
    } catch (reason) { console.error("[y2d-setup] folder picker failed", reason); error = String(reason); }
  }

  async function install() {
    if (busy) return;
    console.info("[y2d-setup] install selected");
    busy = true;
    error = "";
    try { await invoke("setup_install", { destination, shortcuts }); done = true; }
    catch (reason) { console.error("[y2d-setup] install failed", reason); error = String(reason); }
    finally { busy = false; }
  }

  async function close() {
    console.info("[y2d-setup] closing setup");
    if (!busy) await getCurrentWindow().close();
  }

  async function launch() {
    console.info("[y2d-setup] launch selected");
    try { await invoke("setup_launch"); await close(); }
    catch (reason) { console.error("[y2d-setup] launch failed", reason); error = String(reason); }
  }
</script>

<div class="app-shell setup-shell">
  <header class="titlebar" data-tauri-drag-region>
    <div class="titlebar-brand" data-tauri-drag-region>
      <span class="brand-mark" aria-hidden="true">↓</span><span>y2d</span><span class="brand-caption">Setup</span>
    </div>
    <button class="window-close" aria-label="Close setup" disabled={busy} onclick={close}>×</button>
  </header>
  <main class="setup-content">
    <div class="setup-heading">
      <h1>{done ? "Ready to download." : "A home for downloads."}</h1>
      <p>{done ? "y2d is installed and ready to open." : `Install y2d ${version} on this PC.`}</p>
    </div>
    {#if done}
      <div class="setup-complete"><span aria-hidden="true">✓</span><div><strong>Installation complete</strong><p>Video, audio, and all the tools included.</p></div></div>
    {:else}
      <label class="setup-label" for="install-path">Install location</label>
      <div class="setup-path-row">
        <input id="install-path" bind:value={destination} disabled={busy} spellcheck="false" aria-label="Install location" />
        <button class="secondary-button" disabled={busy} onclick={chooseFolder}>Browse</button>
      </div>
      <label class="setup-check"><input type="checkbox" bind:checked={shortcuts} disabled={busy} />Create desktop and Start menu shortcuts</label>
      <div class="setup-included"><span>Included</span><strong>yt-dlp</strong><strong>FFmpeg</strong><strong>ffprobe</strong></div>
    {/if}
    <div class="setup-status" aria-live="polite" aria-busy={busy}>
      {#if error}<p class="inline-error" role="alert">{error}</p>
      {:else if busy}<div class="setup-working"><span></span></div><p>Installing y2d and its tools. Keep this window open.</p>
      {:else if !done}<p>Installs for the current Windows account.</p>{/if}
    </div>
    <div class="setup-actions">
      <button class="text-button" disabled={busy} onclick={close}>{done ? "Close" : "Cancel"}</button>
      <button class="primary-button" disabled={busy || (!done && !destination)} onclick={done ? launch : install}>{done ? "Open y2d" : busy ? "Installing…" : "Install y2d"}</button>
    </div>
  </main>
</div>

<style>
  .setup-content { flex: 1; display: flex; flex-direction: column; padding: 26px 28px 22px; min-height: 0; }
  .setup-heading h1 { font-size: 25px; letter-spacing: -1.2px; }
  .setup-heading p { color: var(--muted); font-size: 11px; margin: 9px 0 24px; }
  .setup-label { color: #bbc8c4; font-size: 11px; margin-bottom: 8px; }
  .setup-path-row { display: flex; gap: 8px; }
  .setup-path-row input { min-width: 0; flex: 1; padding: 10px; border: 1px solid var(--line); border-radius: 5px; color: var(--ink); background: var(--surface-input); font-size: 11px; }
  .setup-check { display: flex; align-items: center; gap: 8px; margin-top: 14px; font-size: 11px; color: #bbc8c4; }
  .setup-check input { accent-color: var(--teal); width: 15px; height: 15px; margin: 0; }
  .setup-included { display: flex; align-items: center; gap: 13px; margin-top: 21px; font-size: 10px; color: var(--muted); }
  .setup-included strong { color: #bcd5cb; font-weight: 400; }
  .setup-status { flex: 1; min-height: 46px; padding-top: 14px; overflow: auto; }
  .setup-status p { font-size: 10px; line-height: 1.5; margin: 6px 0; color: var(--muted); }
  .setup-status .inline-error { color: var(--danger); }
  .setup-actions { display: flex; align-items: center; justify-content: flex-end; gap: 10px; padding-top: 12px; border-top: 1px solid var(--line-soft); }
  .setup-actions .primary-button { margin-left: 0; }
  .setup-complete { display: flex; align-items: center; gap: 17px; padding: 22px 0; color: var(--teal); }
  .setup-complete > span { display: grid; place-items: center; width: 43px; height: 43px; border-radius: 50%; background: #192f2a; font-size: 24px; }
  .setup-complete strong { font-size: 14px; font-weight: 500; }
  .setup-complete p { margin: 8px 0 0; font-size: 10px; color: var(--muted); }
  .setup-working { height: 3px; overflow: hidden; background: var(--line); border-radius: 2px; }
  .setup-working span { display: block; height: 100%; width: 32%; background: var(--teal); animation: working 1.5s ease-in-out infinite alternate; }
  .window-close:disabled { opacity: .3; cursor: wait; }
  @keyframes working { to { transform: translateX(212%); } }
  @media (prefers-reduced-motion: reduce) { .setup-working span { animation: none; width: 100%; opacity: .65; } }
</style>
