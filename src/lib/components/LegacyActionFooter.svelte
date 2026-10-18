<script lang="ts">
  import { systemStore } from '../stores/system';
  import {
    legacyStatusStore,
    isLegacyProcessingStore,
    legacyItemsStore,
    legacyActiveFolderStore,
    legacyStatsStore,
    convertLegacyAll,
  } from '../stores/queue';
  import { legacySettingsStore } from '../stores/settings';
  import { RefreshCw, FileDown } from '@lucide/svelte';

  $: hasConvertibleItems =
    $legacyItemsStore.some((i) => i.status !== 'done') ||
    ($legacyActiveFolderStore !== null && $legacyActiveFolderStore.image_count > 0);
  $: targetFormatUpper = $legacySettingsStore.target_format.toUpperCase();
</script>

<div class="glass-panel p-3 rounded-2xl flex items-center justify-between shrink-0 select-none gap-4">
  <!-- System thread & worker status -->
  <div class="flex items-center space-x-3 text-xs text-slate-400 min-w-0">
    <div class="flex items-center space-x-1.5 shrink-0">
      <span class="w-2 h-2 rounded-full bg-amber-400 animate-pulse"></span>
      <span>
        Decoder: <b class="text-slate-200 font-mono">{$systemStore.threads} threads</b>
      </span>
    </div>
    <span class="text-slate-700 shrink-0">|</span>
    <span class="text-slate-300 truncate">{$legacyStatusStore}</span>
  </div>

  <!-- Status of file size changes: Initial (Red) to New Converted (Green) in MB -->
  <div class="flex items-center space-x-2 px-3.5 py-1.5 rounded-xl bg-surface-900/80 border border-white/5 text-xs shadow-inner shrink-0">
    <span class="text-slate-400 font-medium">Batch Size:</span>
    <span class="text-red-400 font-mono font-bold">{$legacyStatsStore.totalInitialMB} MB</span>
    <span class="text-slate-500 font-bold">→</span>
    <span class="text-emerald-400 font-mono font-bold">{$legacyStatsStore.convertedMB} MB</span>
    {#if $legacyStatsStore.completedCount > 0}
      <span class="text-[10px] font-mono px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 font-bold border border-amber-500/20">
        {$legacyStatsStore.completedCount} converted
      </span>
    {/if}
  </div>

  <!-- Primary CTA Unpack Button -->
  <div class="flex items-center space-x-3 shrink-0">
    <button
      on:click={convertLegacyAll}
      disabled={!hasConvertibleItems || $isLegacyProcessingStore}
      class="px-5 py-2.5 rounded-xl bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-400 hover:to-orange-400 text-surface-950 font-bold text-xs shadow-lg shadow-amber-500/20 transition-all flex items-center space-x-2 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer active:scale-95"
    >
      <FileDown
        class={`w-4 h-4 text-surface-950 ${$isLegacyProcessingStore ? 'animate-bounce' : ''}`}
      />
      <span>Unpack All to .{targetFormatUpper}</span>
    </button>
  </div>
</div>
