<script lang="ts">
  import {
    addFolderFromPath,
    handleDroppedPaths,
    activeFolderStore,
    clearActiveFolder,
    dragStateStore,
    isProcessingStore,
    folderProgressStore,
    formatBytes,
  } from '../stores/queue';
  import { FolderOpen, FolderSync, FolderCheck, ArrowRight, X } from '@lucide/svelte';

  let isDragOver = false;
  let isScanning = false;
  let folderInput: HTMLInputElement;

  $: isTauriOver = $dragStateStore.isDragging && $dragStateStore.targetZone === 'folder';
  $: effectiveDragOver = isDragOver || isTauriOver;

  async function handleSelectFolder() {
    if ($isProcessingStore || isScanning) return;

    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selected = await invoke<string | null>('cmd_select_folder');
      if (selected) {
        isScanning = true;
        await addFolderFromPath(selected);
        isScanning = false;
      }
    } catch (err) {
      console.warn('Native folder picker fallback to input:', err);
      folderInput?.click();
    }
  }

  function handleHtmlFolder(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const files = Array.from(target.files);
      const paths = files.map((f) => (f as any).path || f.name);
      handleDroppedPaths(paths, 'folder');
    }
  }

  function onDragOver(e: DragEvent) {
    e.preventDefault();
    isDragOver = true;
  }

  function onDragLeave() {
    isDragOver = false;
  }

  async function onDrop(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    if ($isProcessingStore || isScanning) return;

    if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
      const paths = Array.from(e.dataTransfer.files).map((f) => (f as any).path || f.name);
      await handleDroppedPaths(paths, 'folder');
    }
  }
</script>

<input
  type="file"
  bind:this={folderInput}
  on:change={handleHtmlFolder}
  webkitdirectory
  multiple
  class="hidden"
/>

<div
  id="dropzone-folder"
  role="button"
  tabindex="0"
  on:click={handleSelectFolder}
  on:keydown={(e) => e.key === 'Enter' && handleSelectFolder()}
  on:dragover={onDragOver}
  on:dragleave={onDragLeave}
  on:drop={onDrop}
  class={`border-2 border-dashed rounded-2xl p-4 transition-all duration-200 cursor-pointer bg-surface-850/40 hover:bg-surface-850/80 flex items-center justify-between group relative overflow-hidden select-none ${
    effectiveDragOver
      ? 'border-amber-400 bg-amber-500/15 scale-[1.005] shadow-lg shadow-amber-500/15'
      : $activeFolderStore
        ? 'border-amber-500/50 bg-amber-500/5 hover:border-amber-400'
        : 'border-slate-700/70 hover:border-amber-400/50'
  } ${$isProcessingStore ? 'opacity-85 cursor-not-allowed' : ''}`}
>
  <div class="flex items-center space-x-3.5 min-w-0 flex-1 mr-3">
    <div
      class={`w-10 h-10 rounded-xl border flex items-center justify-center shrink-0 transition-transform duration-200 shadow-sm ${
        $activeFolderStore
          ? 'bg-amber-500/15 border-amber-500/30 text-amber-400 shadow-amber-500/10'
          : 'bg-gradient-to-br from-amber-500/10 to-orange-500/10 border-amber-500/20 text-amber-400 shadow-amber-500/10 group-hover:scale-105'
      }`}
    >
      {#if isScanning}
        <FolderSync class="w-5 h-5 animate-spin" />
      {:else if $activeFolderStore}
        <FolderCheck class="w-5 h-5" />
      {:else}
        <FolderOpen class="w-5 h-5" />
      {/if}
    </div>

    <div class="min-w-0 flex-1">
      {#if $activeFolderStore}
        <div class="flex items-center space-x-2">
          <span class="text-xs font-semibold text-slate-200">Directory Selected</span>
          <span class="text-[10px] px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 font-mono font-bold border border-amber-500/30">
            {$activeFolderStore.image_count.toLocaleString()} image{$activeFolderStore.image_count === 1 ? '' : 's'}
          </span>
          <span class="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-300 font-mono border border-white/5">
            {formatBytes($activeFolderStore.total_bytes)}
          </span>
        </div>

        <p class="text-[11px] text-amber-300/90 font-mono mt-0.5 truncate max-w-lg" title={$activeFolderStore.folder_path}>
          {$activeFolderStore.folder_path}
        </p>

        {#if $folderProgressStore}
          <div class="mt-2 space-y-1 max-w-md">
            <div class="flex items-center justify-between text-[10px] font-mono text-slate-400">
              <span class="truncate pr-2">{$folderProgressStore.current_file}</span>
              <span class="text-amber-400 font-bold shrink-0">
                {$folderProgressStore.completed} / {$folderProgressStore.total}
              </span>
            </div>
            <div class="w-full h-1.5 bg-slate-800 rounded-full overflow-hidden border border-white/5">
              <div
                class="h-full bg-gradient-to-r from-amber-500 to-orange-500 transition-all duration-150"
                style={`width: ${Math.round(($folderProgressStore.completed / Math.max(1, $folderProgressStore.total)) * 100)}%`}
              ></div>
            </div>
          </div>
        {/if}
      {:else}
        <div class="flex items-center space-x-2">
          <p class="text-xs font-semibold text-slate-200">
            Drop Folder here, or <span class="text-amber-400 underline underline-offset-2">click to set folder path</span>
          </p>
          <span class="text-[9px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-amber-500/10 text-amber-400 font-mono border border-amber-500/20">
            Batch Folder
          </span>
        </div>
        <p class="text-[11px] text-slate-400 mt-0.5">
          Fast directory ingestion without thumbnail loading stall (converts 10,000+ images smoothly)
        </p>
      {/if}
    </div>
  </div>

  <div class="flex items-center space-x-2 shrink-0 text-xs">
    {#if $activeFolderStore && !$isProcessingStore}
      <button
        type="button"
        on:click|stopPropagation={clearActiveFolder}
        title="Clear selected folder"
        class="p-1.5 rounded-lg bg-slate-800/80 hover:bg-red-500/20 text-slate-400 hover:text-red-400 border border-white/5 transition-colors cursor-pointer"
      >
        <X class="w-3.5 h-3.5" />
      </button>
    {/if}

    <span class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 border border-white/5 font-medium transition-colors hidden sm:flex items-center space-x-1.5">
      <span>{$activeFolderStore ? 'Change Folder' : 'Set Folder Path'}</span>
      <ArrowRight class="w-3 h-3 text-slate-400 group-hover:translate-x-0.5 transition-transform" />
    </span>
  </div>
</div>
