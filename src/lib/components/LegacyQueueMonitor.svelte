<script lang="ts">
  import { legacyStatsStore, throughputStore } from '../stores/queue';
  import { History } from '@lucide/svelte';
</script>

<div class="flex items-center justify-between px-2 pt-1 select-none">
  <div class="flex items-center space-x-3">
    <h2 class="text-sm font-semibold tracking-wide text-slate-200 uppercase text-[11px] flex items-center space-x-1.5">
      <History class="w-3.5 h-3.5 text-amber-400" />
      <span>Unpack Queue</span>
    </h2>
    <span class="text-xs bg-slate-800 text-slate-300 font-mono px-2 py-0.5 rounded-full border border-white/5">
      {$legacyStatsStore.totalCount} items
    </span>
  </div>

  <div class="flex items-center space-x-4 text-xs font-mono text-slate-400">
    <div class="flex items-center space-x-1.5">
      <span class="text-slate-400">Completed:</span>
      <span class="text-emerald-400 font-bold">{$legacyStatsStore.completedCount} / {$legacyStatsStore.totalCount}</span>
    </div>
  </div>
</div>
