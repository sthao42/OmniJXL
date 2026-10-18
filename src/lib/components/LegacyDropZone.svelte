<script lang="ts">
  import {
    addJxlFilesFromPaths,
    handleDroppedPaths,
    dragStateStore,
    isLegacyProcessingStore,
  } from '../stores/queue';
  import { FileDown, Layers, Sparkles } from '@lucide/svelte';

  let isDragOver = false;
  let fileInput: HTMLInputElement;

  $: isTauriOver =
    $dragStateStore.isDragging &&
    ($dragStateStore.targetZone === 'files' || $dragStateStore.targetZone === 'workspace');
  $: effectiveDragOver = isDragOver || isTauriOver;

  async function handleBrowse() {
    if ($isLegacyProcessingStore) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selected = await invoke<string[]>('cmd_select_jxl_files');
      if (selected && selected.length > 0) {
        await addJxlFilesFromPaths(selected);
      }
    } catch {
      fileInput?.click();
    }
  }

  function handleHtmlFiles(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const paths = Array.from(target.files).map((f) => (f as any).path || f.name);
      handleDroppedPaths(paths, 'files');
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    isDragOver = true;
  }

  function onDragLeave() {
    isDragOver = false;
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      const paths = Array.from(e.dataTransfer.files).map((f) => (f as any).path || f.name);
      handleDroppedPaths(paths, 'files');
    }
  }
</script>

<div
  id="legacy-dropzone-files"
  role="button"
  tabindex="0"
  on:click={handleBrowse}
  on:keydown={(e) => e.key === 'Enter' && handleBrowse()}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
  class={`file-dropzone border-2 border-dashed rounded-2xl p-6 transition-all duration-200 cursor-pointer text-center flex flex-col items-center justify-center group relative overflow-hidden ${
    effectiveDragOver
      ? 'border-amber-400 bg-amber-500/10 scale-[1.005] shadow-lg shadow-amber-500/15 dragover'
      : 'border-slate-700/80 hover:border-amber-400/50 bg-surface-850/40 hover:bg-surface-850/80'
  }`}
>
  <input
    type="file"
    bind:this={fileInput}
    on:change={handleHtmlFiles}
    multiple
    accept=".jxl,image/jxl"
    class="hidden"
  />

  <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-amber-500/10 to-orange-500/10 border border-amber-500/20 flex items-center justify-center mb-3 group-hover:scale-105 transition-transform duration-200">
    <FileDown class="w-7 h-7 text-amber-400" />
  </div>

  <p class="text-sm font-medium text-slate-200">
    Drag & drop <span class="text-amber-400 font-semibold font-mono">.jxl</span> images here, or <span class="text-amber-400 underline underline-offset-2">browse files</span>
  </p>

  <p class="text-xs text-slate-400 mt-1">
    Unpack JPEG XL back to PNG or JPG for external software and workflows
  </p>

  <!-- Capability Badges -->
  <div class="flex flex-wrap items-center justify-center gap-1.5 mt-3 text-[10px] text-slate-400">
    <span class="px-1.5 py-0.5 rounded bg-purple-950/80 text-purple-300 border border-purple-500/40 font-mono">
      ComfyUI Workflow Recovery
    </span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">
      Alpha & Transparency Intact
    </span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">
      Lossless JPEG Transcode Restore
    </span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">
      ICC & EXIF Preserved
    </span>
  </div>
</div>
