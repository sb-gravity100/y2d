<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { readText } from "@tauri-apps/plugin-clipboard-manager";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
  import { openUrl } from "@tauri-apps/plugin-opener";

  type View = "downloads" | "settings";
  type Mode = "video" | "audio";
  type JobStatus = "queued" | "downloading" | "completed" | "failed" | "cancelled";
  type ToolName = "yt-dlp" | "ffmpeg";
  type NotificationKind = "complete" | "failure";

  function storedNotification(key: string) {
    try { return localStorage.getItem(key) === "true"; }
    catch { return false; }
  }

  type DownloadJob = {
    id: string;
    url: string;
    title: string;
    status: JobStatus;
    progress: number;
    speed?: string;
    eta?: string;
    filePath?: string;
    message?: string;
    mode: Mode;
  };

  type DownloadEvent = Partial<DownloadJob> & { id: string; status: JobStatus };

  type ToolInfo = {
    installed: boolean;
    installedVersion?: string;
    installedSource: string;
    latestVersion?: string;
    latestSizeBytes?: number;
    updateAvailable?: boolean;
    error?: string;
  };

  type ToolsStatus = {
    ytDlp: ToolInfo;
    ffmpeg: ToolInfo;
  };

  type ToolDownloadProgress = {
    tool: ToolName;
    state: string;
    progress: number;
    downloadedBytes: number;
    totalBytes: number;
    message?: string;
  };

  const loadingFolder = "Loading download folder…";

  let activeView = $state<View>("downloads");
  let historyOpen = $state(false);
  let mode = $state<Mode>("video");
  let url = $state("");
  let notifyOnComplete = $state(storedNotification("y2d.notify.complete"));
  let notifyOnFailure = $state(storedNotification("y2d.notify.failure"));
  let downloadDir = $state(loadingFolder);
  let jobs = $state<DownloadJob[]>([]);
  let notice = $state("");
  let submitting = $state(false);
  let tools = $state<ToolsStatus | null>(null);
  let toolsLoading = $state(false);
  let toolsError = $state("");
  let installingTool = $state<ToolName | null>(null);
  let toolProgress = $state<Record<ToolName, ToolDownloadProgress | null>>({
    "yt-dlp": null,
    ffmpeg: null,
  });

  let activeJobs = $derived(jobs.filter((job) => job.status === "queued" || job.status === "downloading"));
  let historyJobs = $derived(jobs.filter((job) => job.status === "completed" || job.status === "failed" || job.status === "cancelled"));
  let toolRows = $derived(tools ? [
    { id: "yt-dlp" as const, title: "yt-dlp", info: tools.ytDlp, subtitle: "Video and audio downloads" },
    { id: "ffmpeg" as const, title: "FFmpeg", info: tools.ffmpeg, subtitle: "Audio conversion and media embedding" },
  ] : []);

  onMount(() => {
    let disposed = false;
    let stopJobEvents: (() => void) | undefined;
    let stopToolEvents: (() => void) | undefined;

    void autoPasteClipboard();
    void refreshTools();

    listen<DownloadEvent>("download-event", ({ payload }) => {
      if (payload.status !== "downloading" || payload.progress === undefined) {
        console.debug("[y2d] download event", payload.id, payload.status);
      }
      const existing = jobs.find((job) => job.id === payload.id);
      if (payload.status === "completed" || payload.status === "failed") {
        void notifyDownload(payload.status, existing?.title || payload.title || "Download");
      }
      if (existing) {
        jobs = jobs.map((job) => job.id === payload.id
          ? {
              ...job,
              ...payload,
              title: payload.title || job.title,
              url: payload.url || job.url,
              mode: payload.mode || job.mode,
              progress: payload.progress ?? job.progress,
            }
          : job);
        return;
      }

      jobs = [{
        id: payload.id,
        url: payload.url || "",
        title: payload.title || "Preparing download",
        status: payload.status,
        progress: payload.progress ?? 0,
        speed: payload.speed,
        eta: payload.eta,
        filePath: payload.filePath,
        message: payload.message,
        mode: payload.mode || "video",
      }, ...jobs];
    }).then((stop) => {
      if (disposed) stop();
      else stopJobEvents = stop;
    }).catch((error) => {
      console.error("[y2d] download event listener failed", error);
      if (!disposed) notice = "Download updates could not be connected.";
    });

    listen<ToolDownloadProgress>("tool-download-progress", ({ payload }) => {
      toolProgress = { ...toolProgress, [payload.tool]: payload };
    }).then((stop) => {
      if (disposed) stop();
      else stopToolEvents = stop;
    }).catch((error) => {
      console.error("[y2d] tool progress listener failed", error);
      if (!disposed) toolsError = "Tool download progress could not be connected.";
    });

    invoke<string>("default_download_dir").then((path) => {
      console.info("[y2d] default destination loaded");
      downloadDir = path;
    }).catch((error) => {
      console.error("[y2d] default destination load failed", error);
      downloadDir = "";
      notice = String(error);
    });

    return () => {
      disposed = true;
      stopJobEvents?.();
      stopToolEvents?.();
    };
  });

  async function autoPasteClipboard(force = false) {
    if (!force && url.trim()) return;
    try {
      const text = (await readText()).trim();
      let candidate: URL;
      try { candidate = new URL(text); } catch { return; }
      if ((candidate.protocol !== "http:" && candidate.protocol !== "https:") || !candidate.hostname) return;
      if (!force && url.trim()) return;
      url = text;
      console.info("[y2d] URL detected in clipboard");
    } catch (error) {
      console.debug("[y2d] clipboard read unavailable", error);
    }
  }

  async function notificationPermissionGranted() {
    let granted = await isPermissionGranted();
    if (!granted) granted = (await requestPermission()) === "granted";
    return granted;
  }

  async function setNotificationPreference(kind: NotificationKind, enabled: boolean) {
    try {
      if (enabled && !(await notificationPermissionGranted())) {
        notice = "Allow notifications in Windows to turn this on.";
        return;
      }
      const key = kind === "complete" ? "y2d.notify.complete" : "y2d.notify.failure";
      localStorage.setItem(key, String(enabled));
      if (kind === "complete") notifyOnComplete = enabled;
      else notifyOnFailure = enabled;
    } catch (error) {
      console.error("[y2d] notification preference update failed", error);
      notice = "Could not update notification settings.";
    }
  }

  async function notifyDownload(status: "completed" | "failed", title: string) {
    if (status === "completed" ? !notifyOnComplete : !notifyOnFailure) return;
    try {
      if (!(await isPermissionGranted())) return;
      sendNotification({
        title: status === "completed" ? "Download complete" : "Download failed",
        body: title,
      });
    } catch (error) {
      console.error("[y2d] desktop notification failed", error);
    }
  }

  async function testNotification() {
    try {
      if (!(await notificationPermissionGranted())) return;
      sendNotification({ title: "y2d", body: "Notifications are on." });
    } catch (error) {
      console.error("[y2d] test notification failed", error);
      notice = "Could not send a test notification.";
    }
  }

  async function openCutv() {
    try {
      await openUrl("https://github.com/sb-gravity100/cutv_rust/releases/latest");
    } catch (error) {
      console.error("[y2d] could not open CUTV release page", error);
      notice = "Could not open CUTV in the browser.";
    }
  }

  function navigate(view: View) {
    activeView = view;
    historyOpen = false;
    if (view === "settings" && !tools && !toolsLoading) void refreshTools();
  }

  function toggleHistory() {
    historyOpen = !historyOpen;
    console.info("[y2d] history drawer", historyOpen ? "opened" : "closed");
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === "Escape" && historyOpen) {
      historyOpen = false;
      console.info("[y2d] history drawer closed with Escape");
    }
  }

  async function startWindowDrag(event: MouseEvent) {
    if (event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest("button")) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("[y2d] window drag failed", error);
    }
  }

  async function closeWindow() {
    console.info("[y2d] window close requested");
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.error("[y2d] window close failed", error);
      notice = "Could not close the window: " + String(error);
    }
  }

  async function chooseFolder() {
    console.info("[y2d] destination picker requested");
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        ...(downloadDir && downloadDir !== loadingFolder ? { defaultPath: downloadDir } : {}),
      });
      if (typeof selected === "string") downloadDir = selected;
    } catch (error) {
      console.error("[y2d] destination picker failed", error);
      notice = "Could not open folder picker: " + String(error);
    }
  }

  async function addDownload(event: SubmitEvent) {
    event.preventDefault();
    const cleanUrl = url.trim();
    console.info("[y2d] download submitted", { mode, hasUrl: Boolean(cleanUrl) });
    if (!cleanUrl || submitting) return;

    const temporaryId = "pending-" + Date.now();
    const optimistic: DownloadJob = {
      id: temporaryId,
      url: cleanUrl,
      title: "Starting download",
      status: "queued",
      progress: 0,
      mode,
    };
    jobs = [optimistic, ...jobs];
    url = "";
    submitting = true;
    notice = "";

    try {
      const id = await invoke<string>("start_download", {
        request: { url: cleanUrl, outputDir: downloadDir, mode },
      });
      jobs = jobs.some((job) => job.id === id)
        ? jobs.filter((job) => job.id !== temporaryId)
        : jobs.map((job) => job.id === temporaryId ? { ...job, id } : job);
    } catch (error) {
      console.error("[y2d] could not start download", error);
      jobs = jobs.map((job) => job.id === temporaryId
        ? { ...job, status: "failed", message: String(error), title: "Could not start download" }
        : job);
      notice = String(error);
    } finally {
      submitting = false;
    }
  }

  async function cancelDownload(id: string) {
    console.info("[y2d] cancel requested", id);
    try {
      await invoke("cancel_download", { id });
    } catch (error) {
      console.error("[y2d] could not cancel download", id, error);
      notice = "Could not cancel download: " + String(error);
    }
  }

  async function revealFile(filePath?: string) {
    if (!filePath) return;
    console.info("[y2d] reveal requested");
    try {
      await invoke("reveal_file", { filePath });
    } catch (error) {
      console.error("[y2d] could not reveal file", error);
      notice = "Could not reveal file: " + String(error);
    }
  }

  async function refreshTools() {
    console.info("[y2d] refreshing managed tool status");
    toolsLoading = true;
    toolsError = "";
    try {
      tools = await invoke<ToolsStatus>("get_tool_status");
      if (tools.ytDlp.error) toolsError = tools.ytDlp.error;
      if (tools.ffmpeg.error) toolsError = [toolsError, tools.ffmpeg.error].filter(Boolean).join(" ");
    } catch (error) {
      console.error("[y2d] could not read tool status", error);
      toolsError = String(error);
    } finally {
      toolsLoading = false;
    }
  }

  async function installTool(tool: ToolName) {
    if (installingTool) return;
    console.info("[y2d] tool install requested", tool);
    installingTool = tool;
    toolsError = "";
    const info = tool === "yt-dlp" ? tools?.ytDlp : tools?.ffmpeg;
    toolProgress = {
      ...toolProgress,
      [tool]: {
        tool,
        state: "starting",
        progress: 0,
        downloadedBytes: 0,
        totalBytes: info?.latestSizeBytes || 0,
      },
    };
    try {
      tools = await invoke<ToolsStatus>("install_tool", { tool });
      toolProgress = {
        ...toolProgress,
        [tool]: { tool, state: "complete", progress: 100, downloadedBytes: 0, totalBytes: 0 },
      };
    } catch (error) {
      console.error("[y2d] tool install failed", tool, error);
      toolsError = String(error);
      toolProgress = {
        ...toolProgress,
        [tool]: { tool, state: "failed", progress: 0, downloadedBytes: 0, totalBytes: 0, message: String(error) },
      };
    } finally {
      installingTool = null;
    }
  }

  function shortPath(path: string) {
    if (!path) return "Choose a folder";
    if (path === loadingFolder) return path;
    return path.replace(/^.*\\Downloads(?=\\|$)/i, "Downloads");
  }

  function statusLabel(status: JobStatus) {
    return {
      queued: "Queued",
      downloading: "Downloading",
      completed: "Complete",
      failed: "Failed",
      cancelled: "Cancelled",
    }[status];
  }

  function sourceLabel(source: string) {
    return {
      managed: "Managed by y2d",
      bundled: "Bundled fallback",
      system: "Found on system PATH",
      missing: "Not installed",
    }[source] || source;
  }

  function actionLabel(tool: ToolName, info: ToolInfo) {
    if (installingTool === tool) return "Installing…";
    if (tool === "ffmpeg" && info.installedSource !== "managed") return "Download copy";
    if (info.updateAvailable === false) return "Up to date";
    if (info.installed) return "Update";
    return "Download";
  }

  function formatBytes(bytes?: number) {
    if (!bytes) return "";
    return (bytes / (1024 * 1024)).toFixed(1) + " MiB";
  }

  function displaySource(version?: string, source?: string) {
    if (!version) return sourceLabel(source || "missing");
    return version + " · " + sourceLabel(source || "missing");
  }
</script>

<svelte:head>
  <title>y2d · Download Manager</title>
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} onfocus={() => void autoPasteClipboard()} />

<div class="app-shell">
  <header class="titlebar" role="toolbar" aria-label="Window controls and drag area" tabindex="0" onmousedown={startWindowDrag}>
    <div class="titlebar-brand"><span class="brand-mark">↓</span><span>y2d</span><span class="brand-caption">DOWNLOAD DESK</span></div>
    <button class="window-close" aria-label="Close window" title="Close window" onclick={closeWindow}>×</button>
  </header>

  <main class="main">
    <section class="content">
      {#if notice}
        <div class="notice" role="status"><span>{notice}</span><button aria-label="Dismiss message" onclick={() => notice = ""}>×</button></div>
      {/if}

      {#if activeView === "settings"}
        <div class="page-head">
          <div><h1>Settings</h1><p>Destination and download tools.</p></div>
          <button class="quiet-button" onclick={() => navigate("downloads")}>Back to downloads</button>
        </div>

        <section class="settings-group">
          <div class="group-head">
            <div><h2>Download location</h2><p>New files will be saved here.</p></div>
            <button class="secondary-button" onclick={chooseFolder}>Choose folder</button>
          </div>
          <div class="path-field">{downloadDir || "No folder selected"}</div>
        </section>

        <section class="settings-group tool-group">
          <div class="group-head">
            <div><h2>Download tools</h2><p>Verified downloads in y2d. No system PATH changes or admin access.</p></div>
            <button class="text-button" disabled={toolsLoading || installingTool !== null} onclick={refreshTools}>
              {toolsLoading ? "Checking…" : "Refresh"}
            </button>
          </div>

          {#if toolsLoading && !tools}
            <div class="loading-line">Checking installed versions…</div>
          {:else if tools}
            <div class="tool-list">
              {#each toolRows as tool (tool.id)}
                <article class="tool-row">
                  <div class="tool-details">
                    <div class="tool-name">{tool.title}</div>
                    <div class="tool-subtitle">{tool.subtitle}</div>
                    <div class="tool-version">
                      {tool.info.installed ? displaySource(tool.info.installedVersion, tool.info.installedSource) : "Not installed"}
                      {#if tool.info.latestVersion}<span class="latest-version">Latest {tool.info.latestVersion}</span>{/if}
                      {#if tool.info.latestSizeBytes}<span class="download-size">{formatBytes(tool.info.latestSizeBytes)} download</span>{/if}
                    </div>
                    {#if toolProgress[tool.id] && installingTool === tool.id}
                      <div class="tool-progress"><span style="width: {toolProgress[tool.id]?.progress || 0}%"></span></div>
                    {/if}
                  </div>
                  <button
                    class="tool-action"
                    disabled={installingTool !== null || toolsLoading || !tool.info.latestVersion || tool.info.updateAvailable === false}
                    onclick={() => installTool(tool.id)}
                  >{actionLabel(tool.id, tool.info)}</button>
                </article>
              {/each}
            </div>
          {:else}
            <div class="loading-line">Tool status is not available yet.</div>
          {/if}
          {#if toolsError}<p class="inline-error">{toolsError}</p>{/if}
        </section>

        <section class="settings-group notification-group">
          <div class="group-head">
            <div><h2>Notifications</h2><p>Desktop alerts for finished downloads.</p></div>
            <button class="text-button" onclick={testNotification}>Test</button>
          </div>
          <label class="preference-row">
            <span>Download complete</span>
            <input type="checkbox" checked={notifyOnComplete} onchange={(event) => void setNotificationPreference("complete", event.currentTarget.checked)} />
          </label>
          <label class="preference-row">
            <span>Download failed</span>
            <input type="checkbox" checked={notifyOnFailure} onchange={(event) => void setNotificationPreference("failure", event.currentTarget.checked)} />
          </label>
        </section>

        <section class="settings-group about-group">
          <div class="group-head"><div><h2>About y2d</h2><p>Version 0.1.0 · Windows</p></div><span class="release-tag">Early access</span></div>
        </section>
      {:else}
        <div class="page-head">
          <div>
            <h1>Downloads</h1>
          </div>
          <div class="page-actions">
            <button class="quiet-button" aria-expanded={historyOpen} onclick={toggleHistory}>History <span>{historyJobs.length}</span></button>
            <button class="quiet-button" onclick={() => navigate("settings")}>Settings</button>
          </div>
        </div>

        <form class="composer" onsubmit={addDownload}>
          <label for="download-url">URL</label>
          <div class="url-field">
            <input id="download-url" bind:value={url} type="url" placeholder="https://…" required />
            <button type="button" class="paste-button" onclick={() => void autoPasteClipboard(true)}>Paste</button>
          </div>
          <div class="composer-bottom">
            <div class="format-switch" aria-label="Download format">
              <button type="button" class:selected={mode === "video"} aria-pressed={mode === "video"} onclick={() => mode = "video"}>Video</button>
              <button type="button" class:selected={mode === "audio"} aria-pressed={mode === "audio"} onclick={() => mode = "audio"}>MP3 audio</button>
            </div>
            <button type="button" class="folder-choice" title={downloadDir} onclick={chooseFolder}>{shortPath(downloadDir)}</button>
            <button class="primary-button" type="submit" disabled={submitting || !url.trim() || !downloadDir || downloadDir === loadingFolder}>
              {submitting ? "Adding…" : "Download"}
            </button>
          </div>
        </form>

        {#if tools?.ytDlp && !tools.ytDlp.installed}
          <div class="tool-warning"><span>yt-dlp is missing</span><button onclick={() => navigate("settings")}>Install</button></div>
        {/if}
        {#if tools?.ffmpeg && !tools.ffmpeg.installed}
          <div class="tool-warning"><span>FFmpeg is missing · audio and merging may fail</span><button onclick={() => navigate("settings")}>Install</button></div>
        {/if}

        <div class="list-head">
          <h2>Current queue</h2>
          <span>{activeJobs.length.toString().padStart(2, "0")}</span>
        </div>

        {#if activeJobs.length === 0}
          <div class="empty-state">
            <div class="empty-symbol">↓</div>
            <strong>Queue is clear</strong>
          </div>
        {:else}
          <div class="job-list">
            {#each activeJobs as job (job.id)}
              <article class="job-row">
                <div class="job-main">
                  <div class="job-title">
                    <strong>{job.title}</strong>
                    <span class="job-status {job.status}">{statusLabel(job.status)}</span>
                  </div>
                  <div class="job-url" title={job.url}>{job.url || job.message || "Preparing download"}</div>
                  <div class="progress-track"><span style="width: {Math.min(job.progress, 100)}%"></span></div>
                  <div class="job-meta">
                    <span>{job.progress > 0 ? job.progress.toFixed(1) + "%" : "Preparing"}</span>
                    <span>{job.speed || "Waiting for data"}</span>
                    <span>{job.eta ? job.eta + " left" : ""}</span>
                  </div>
                </div>
                {#if !job.id.startsWith("pending-")}
                  <button class="cancel-button" aria-label={"Cancel " + job.title} onclick={() => cancelDownload(job.id)}>Cancel</button>
                {/if}
              </article>
            {/each}
          </div>
        {/if}

        <button class="cutv-promo" onclick={openCutv}>
          <span class="cutv-mark">CUTV</span>
          <span class="cutv-copy">Trim videos · export GIFs</span>
          <span class="cutv-link">Explore ↗</span>
        </button>
      {/if}

    </section>
  </main>

  {#if historyOpen}
    <button class="drawer-scrim" aria-label="Close history drawer" onclick={() => historyOpen = false}></button>
  {/if}
  <div class:open={historyOpen} class="history-drawer" role="dialog" aria-modal={historyOpen} aria-label="Download history" aria-hidden={!historyOpen} inert={!historyOpen}>
    <div class="drawer-head">
      <div><h2>History</h2><p>{historyJobs.length} downloads this session</p></div>
      <button class="drawer-done" onclick={() => historyOpen = false}>Done</button>
    </div>
    {#if historyJobs.length === 0}
      <div class="drawer-empty"><strong>No history yet</strong><p>Completed and stopped downloads appear here.</p></div>
    {:else}
      <div class="drawer-jobs">
        {#each historyJobs as job (job.id)}
          <article class="drawer-job">
            <div class="drawer-job-top">
              <strong>{job.title}</strong>
              <span class="job-status {job.status}">{statusLabel(job.status)}</span>
            </div>
            <p>{job.url || job.message || "Download"}</p>
            {#if job.status === "completed" && job.filePath}
              <button class="text-button" onclick={() => revealFile(job.filePath)}>Show file</button>
            {:else if job.message}
              <span class="drawer-error">{job.message}</span>
            {/if}
          </article>
        {/each}
      </div>
    {/if}
  </div>
</div>
