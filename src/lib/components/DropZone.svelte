<script lang="ts">
  import {
    addFilesFromPaths,
    handleDroppedPaths,
    dragStateStore,
    isProcessingStore,
  } from '../stores/queue';
  import { UploadCloud } from '@lucide/svelte';

  let isDragOver = false;
  let fileInput: HTMLInputElement;

  $: isTauriOver =
    $dragStateStore.isDragging &&
    ($dragStateStore.targetZone === 'files' || $dragStateStore.targetZone === 'workspace');
  $: effectiveDragOver = isDragOver || isTauriOver;

  async function handleBrowse() {
    if ($isProcessingStore) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selected = await invoke<string[]>('cmd_select_files');
      if (selected && selected.length > 0) {
        await addFilesFromPaths(selected);
      }
    } catch {
      // Fallback in web preview mode
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
  id="dropzone-files"
  role="button"
  tabindex="0"
  on:click={handleBrowse}
  on:keydown={(e) => e.key === 'Enter' && handleBrowse()}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
  class={`file-dropzone border-2 border-dashed rounded-2xl p-6 transition-all duration-200 cursor-pointer text-center flex flex-col items-center justify-center group relative overflow-hidden ${
    effectiveDragOver
      ? 'border-jxl-teal bg-jxl-teal/10 scale-[1.005] shadow-lg shadow-jxl-teal/15 dragover'
      : 'border-slate-700/80 hover:border-jxl-teal/50 bg-surface-850/40 hover:bg-surface-850/80'
  }`}
>
  <input
    type="file"
    bind:this={fileInput}
    on:change={handleHtmlFiles}
    multiple
    accept="image/*,.cr2,.nef,.arw,.heic,.avif,.webp,.jxl,.tiff,.tif,.bmp,.gif,.hdr,.exr"
    class="hidden"
  />

  <div class="w-14 h-14 rounded-2xl bg-gradient-to-br from-jxl-teal/10 to-brand-500/10 border border-white/10 flex items-center justify-center mb-3 group-hover:scale-105 transition-transform duration-200">
    <UploadCloud class="w-7 h-7 text-jxl-teal" />
  </div>

  <p class="text-sm font-medium text-slate-200">
    Drag & drop images here, or <span class="text-jxl-teal underline underline-offset-2">browse files</span>
  </p>

  <p class="text-xs text-slate-400 mt-1">
    Supports PNG, JPEG, WebP, AVIF, TIFF, BMP, GIF, HDR & RAW
  </p>

  <!-- Format Badges -->
  <div class="flex flex-wrap items-center justify-center gap-1.5 mt-3 text-[10px] text-slate-400">
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">PNG (Lossless)</span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">JPEG (Reversible)</span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">AVIF / WebP</span>
    <span class="px-1.5 py-0.5 rounded bg-slate-800 border border-white/5">16-bit Float</span>
  </div>
</div>
