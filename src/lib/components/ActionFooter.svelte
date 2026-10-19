<script lang="ts">
  import { systemStore } from '../stores/system';
  import {
    overallStatusStore,
    isProcessingStore,
    itemsStore,
    activeFolderStore,
    queueStatsStore,
    convertAll,
    cancelBatch,
  } from '../stores/queue';
  import { RefreshCw, Square } from '@lucide/svelte';

  $: hasConvertibleItems =
    $itemsStore.some((i) => i.status !== 'done') ||
    ($activeFolderStore !== null && $activeFolderStore.image_count > 0);
</script>

<div class="glass-panel p-3 rounded-2xl flex items-center justify-between shrink-0 select-none gap-4">
  <!-- System thread & worker status -->
  <div class="flex items-center space-x-3 text-xs text-slate-400 min-w-0">
    <div class="flex items-center space-x-1.5 shrink-0">
      <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
      <span>
        Workers: <b class="text-slate-200 font-mono">{$systemStore.threads} threads</b>
      </span>
    </div>
    <span class="text-slate-700 shrink-0">|</span>
    <span class="text-slate-300 truncate">{$overallStatusStore}</span>
  </div>

  <!-- Status of file size changes: Initial (Red) to New Converted (Green) in MB -->
  <div class="flex items-center space-x-2 px-3.5 py-1.5 rounded-xl bg-surface-900/80 border border-white/5 text-xs shadow-inner shrink-0">
    <span class="text-slate-400 font-medium">Batch Size:</span>
    <span class="text-red-400 font-mono font-bold">{$queueStatsStore.totalInitialMB} MB</span>
    <span class="text-slate-500 font-bold">→</span>
    <span class="text-emerald-400 font-mono font-bold">{$queueStatsStore.convertedMB} MB</span>
    {#if $queueStatsStore.completedCount > 0 && $queueStatsStore.savedPercent > 0}
      <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-emerald-500/15 text-emerald-400 font-bold border border-emerald-500/20">
        -{$queueStatsStore.savedPercent}%
      </span>
    {/if}
  </div>

  <!-- Primary CTA Convert / Cancel Button -->
  <div class="flex items-center space-x-2.5 shrink-0">
    {#if $isProcessingStore}
      <button
        on:click={cancelBatch}
        class="px-4 py-2.5 rounded-xl bg-rose-500/15 hover:bg-rose-500/25 text-rose-300 border border-rose-500/30 font-bold text-xs shadow-lg transition-all flex items-center space-x-2 cursor-pointer active:scale-95"
        title="Immediately cancel remaining batch items"
      >
        <Square class="w-3.5 h-3.5 text-rose-400 fill-rose-400" />
        <span>Stop</span>
      </button>
    {/if}

    <button
      on:click={convertAll}
      disabled={!hasConvertibleItems || $isProcessingStore}
      class="px-5 py-2.5 rounded-xl bg-gradient-to-r from-jxl-teal to-jxl-blue hover:from-teal-300 hover:to-blue-400 text-surface-900 font-bold text-xs shadow-lg shadow-jxl-teal/20 transition-all flex items-center space-x-2 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer active:scale-95"
    >
      <RefreshCw
        class={`w-4 h-4 text-surface-900 ${$isProcessingStore ? 'animate-spin' : ''}`}
      />
      <span>{$isProcessingStore ? 'Encoding...' : 'Convert All to .JXL'}</span>
    </button>
  </div>
</div>
