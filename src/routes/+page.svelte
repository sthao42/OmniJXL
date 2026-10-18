<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Header from '$lib/components/Header.svelte';
  import DropZone from '$lib/components/DropZone.svelte';
  import FolderDropZone from '$lib/components/FolderDropZone.svelte';
  import QueueMonitor from '$lib/components/QueueMonitor.svelte';
  import QueueList from '$lib/components/QueueList.svelte';
  import ActionFooter from '$lib/components/ActionFooter.svelte';
  import Sidebar from '$lib/components/Sidebar.svelte';

  import LegacyDropZone from '$lib/components/LegacyDropZone.svelte';
  import LegacyFolderDropZone from '$lib/components/LegacyFolderDropZone.svelte';
  import LegacyQueueMonitor from '$lib/components/LegacyQueueMonitor.svelte';
  import LegacyQueueList from '$lib/components/LegacyQueueList.svelte';
  import LegacyActionFooter from '$lib/components/LegacyActionFooter.svelte';
  import LegacySidebar from '$lib/components/LegacySidebar.svelte';

  import Toast from '$lib/components/Toast.svelte';

  import { fetchSystemInfo } from '$lib/stores/system';
  import {
    handleProgressEvent,
    handleLegacyProgressEvent,
    handleDroppedPaths,
    dragStateStore,
    queueStatsStore,
    legacyStatsStore,
    overallStatusStore,
    legacyStatusStore,
    activeTabStore,
    folderProgressStore,
    legacyFolderProgressStore,
  } from '$lib/stores/queue';
  import type { ProgressUpdate } from '$lib/types';
  import type { FolderBatchProgress } from '$lib/stores/queue';
  import { get } from 'svelte/store';

  let unlistenProgress: (() => void) | null = null;
  let unlistenLegacyProgress: (() => void) | null = null;
  let unlistenFolderProgress: (() => void) | null = null;
  let unlistenLegacyFolderProgress: (() => void) | null = null;
  let unlistenDragDrop: (() => void) | null = null;

  function determineDropZone(
    x: number,
    y: number,
    tab: 'encode' | 'legacy'
  ): 'files' | 'folder' | 'workspace' {
    const folderId = tab === 'encode' ? 'dropzone-folder' : 'legacy-dropzone-folder';
    const folderElem = document.getElementById(folderId);
    if (folderElem) {
      const rect = folderElem.getBoundingClientRect();
      if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
        return 'folder';
      }
    }

    const fileId = tab === 'encode' ? 'dropzone-files' : 'legacy-dropzone-files';
    const fileElem = document.getElementById(fileId);
    if (fileElem) {
      const rect = fileElem.getBoundingClientRect();
      if (x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom) {
        return 'files';
      }
    }

    return 'workspace';
  }

  onMount(async () => {
    // 1. Fetch live system hardware info (threads, SIMD, libjxl version)
    await fetchSystemInfo();

    // 2. Listen to real-time Tauri conversion progress stream
    try {
      const { listen } = await import('@tauri-apps/api/event');
      unlistenProgress = await listen<ProgressUpdate>('conversion-progress', (event) => {
        handleProgressEvent(event.payload);
      });
      unlistenLegacyProgress = await listen<ProgressUpdate>('legacy-conversion-progress', (event) => {
        handleLegacyProgressEvent(event.payload);
      });
      unlistenFolderProgress = await listen<FolderBatchProgress>('folder-conversion-progress', (event) => {
        folderProgressStore.set(event.payload);
      });
      unlistenLegacyFolderProgress = await listen<FolderBatchProgress>('legacy-folder-conversion-progress', (event) => {
        legacyFolderProgressStore.set(event.payload);
      });
    } catch (err) {
      console.warn('Tauri event listener not available in preview mode:', err);
    }

    // 3. Listen to native Tauri OS Drag and Drop events (files and folders from Windows Explorer)
    try {
      const { getCurrentWebview } = await import('@tauri-apps/api/webview');
      unlistenDragDrop = await getCurrentWebview().onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === 'enter' || payload.type === 'over') {
          const dpr = window.devicePixelRatio || 1;
          const logicalX = payload.position.x / dpr;
          const logicalY = payload.position.y / dpr;
          const zone = determineDropZone(logicalX, logicalY, get(activeTabStore));
          dragStateStore.set({ isDragging: true, targetZone: zone });
        } else if (payload.type === 'leave') {
          dragStateStore.set({ isDragging: false, targetZone: null });
        } else if (payload.type === 'drop') {
          const dpr = window.devicePixelRatio || 1;
          const logicalX = payload.position.x / dpr;
          const logicalY = payload.position.y / dpr;
          const zone = determineDropZone(logicalX, logicalY, get(activeTabStore));
          dragStateStore.set({ isDragging: false, targetZone: null });
          handleDroppedPaths(payload.paths, zone);
        }
      });
    } catch (err) {
      console.warn('Native drag-drop handler not active in preview mode:', err);
    }
  });

  onDestroy(() => {
    if (unlistenProgress) {
      unlistenProgress();
    }
    if (unlistenLegacyProgress) {
      unlistenLegacyProgress();
    }
    if (unlistenFolderProgress) {
      unlistenFolderProgress();
    }
    if (unlistenLegacyFolderProgress) {
      unlistenLegacyFolderProgress();
    }
    if (unlistenDragDrop) {
      unlistenDragDrop();
    }
  });
</script>

<div class="h-screen w-screen flex flex-col overflow-hidden bg-surface-900 text-slate-100 font-sans">
  <!-- Desktop Titlebar Header with Tab Switcher -->
  <Header />

  <!-- Main 2-Column Responsive Workspace -->
  <div class="flex-1 flex overflow-hidden">
    {#if $activeTabStore === 'encode'}
      <!-- Tab 1: Encode to .JXL Workspace -->
      <main class="flex-1 flex flex-col p-4 overflow-hidden gap-3 min-w-0">
        <DropZone />
        <FolderDropZone />
        <QueueMonitor />
        <QueueList />
        <ActionFooter />
      </main>

      <!-- Right Sidebar: JXL Encoder Parameters & Settings -->
      <Sidebar />
    {:else}
      <!-- Tab 2: JXL to Legacy (PNG / JPG) Workspace -->
      <main class="flex-1 flex flex-col p-4 overflow-hidden gap-3 min-w-0">
        <LegacyDropZone />
        <LegacyFolderDropZone />
        <LegacyQueueMonitor />
        <LegacyQueueList />
        <LegacyActionFooter />
      </main>

      <!-- Right Sidebar: Legacy Decoder Parameters & Settings -->
      <LegacySidebar />
    {/if}
  </div>

  <!-- App Window Bottom Status Bar -->
  <footer class="h-7 bg-surface-950 border-t border-white/5 px-4 flex items-center justify-between text-xs text-slate-400 shrink-0 select-none z-10">
    <div class="flex items-center space-x-3 min-w-0">
      <div class="flex items-center space-x-1.5 shrink-0">
        <span class={`w-1.5 h-1.5 rounded-full ${$activeTabStore === 'encode' ? 'bg-emerald-400' : 'bg-amber-400'}`}></span>
        <span class="text-slate-300 font-medium">Status:</span>
      </div>
      <span class="text-slate-300 truncate">
        {$activeTabStore === 'encode' ? $overallStatusStore : $legacyStatusStore}
      </span>
    </div>

    <!-- Status of file size changes: initial size (Red) to new size (Green) in MB -->
    <div class="flex items-center space-x-2 font-medium shrink-0">
      <span class="text-slate-400">File size:</span>
      <span class="text-red-400 font-mono font-bold">
        {$activeTabStore === 'encode' ? $queueStatsStore.totalInitialMB : $legacyStatsStore.totalInitialMB} MB
      </span>
      <span class="text-slate-500 font-bold">→</span>
      <span class="text-emerald-400 font-mono font-bold">
        {$activeTabStore === 'encode' ? $queueStatsStore.convertedMB : $legacyStatsStore.convertedMB} MB
      </span>
      {#if $activeTabStore === 'encode' && $queueStatsStore.completedCount > 0 && $queueStatsStore.savedPercent > 0}
        <span class="text-[10px] font-mono text-emerald-400 bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-500/20 font-bold ml-1">
          -{$queueStatsStore.savedPercent}%
        </span>
      {:else if $activeTabStore === 'legacy' && $legacyStatsStore.completedCount > 0}
        <span class="text-[10px] font-mono text-amber-400 bg-amber-500/10 px-1.5 py-0.5 rounded border border-amber-500/20 font-bold ml-1">
          {$legacyStatsStore.completedCount} / {$legacyStatsStore.totalCount}
        </span>
      {/if}
    </div>
  </footer>

  <!-- Notification Toast -->
  <Toast />
</div>
