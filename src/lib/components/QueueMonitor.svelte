<script lang="ts">
  import { queueStatsStore, throughputStore } from '../stores/queue';
</script>

<div class="flex items-center justify-between px-2 pt-1 select-none">
  <div class="flex items-center space-x-3">
    <h2 class="text-sm font-semibold tracking-wide text-slate-200 uppercase text-[11px]">
      Conversion Queue
    </h2>
    <span class="text-xs bg-slate-800 text-slate-300 font-mono px-2 py-0.5 rounded-full border border-white/5">
      {$queueStatsStore.totalCount} items
    </span>
  </div>

  <!-- Overall Batch Progress Stats -->
  <div class="flex items-center space-x-4 text-xs font-mono text-slate-400">
    <div class="flex items-center space-x-1.5">
      <span class="text-slate-400">Saved:</span>
      <span class="text-emerald-400 font-bold">{$queueStatsStore.savedFormatted}</span>
    </div>
    <div class="flex items-center space-x-1.5">
      <span class="text-slate-400">Speed:</span>
      <span class="text-slate-300 font-semibold">{$throughputStore}</span>
    </div>
  </div>
</div>
