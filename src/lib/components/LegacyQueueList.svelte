<script lang="ts">
  import {
    legacyItemsStore,
    legacyActiveFolderStore,
    legacyFolderProgressStore,
    formatBytes,
  } from '../stores/queue';
  import LegacyQueueItemCard from './LegacyQueueItemCard.svelte';
  import { FileDown, FolderCheck, Zap } from '@lucide/svelte';
</script>

<div class="flex-1 overflow-y-auto space-y-2.5 pr-1 min-h-0">
  {#if $legacyActiveFolderStore}
    <!-- High-Performance JXL Folder Batch Banner -->
    <div class="glass-panel p-4 rounded-2xl border border-amber-500/25 bg-surface-850/70 select-none relative overflow-hidden shadow-lg shadow-black/20">
      <div class="flex items-start justify-between gap-4">
        <div class="flex items-start space-x-3.5 min-w-0">
          <div class="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/25 flex items-center justify-center shrink-0 text-amber-400 mt-0.5">
            <FolderCheck class="w-5 h-5" />
          </div>
          <div class="min-w-0">
            <div class="flex items-center space-x-2 flex-wrap gap-y-1">
              <h3 class="text-xs font-bold text-slate-100 font-mono">
                {$legacyActiveFolderStore.folder_name}
              </h3>
              <span class="text-[10px] px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-300 font-mono font-bold border border-amber-500/30">
                {$legacyActiveFolderStore.image_count.toLocaleString()} .jxl file{$legacyActiveFolderStore.image_count === 1 ? '' : 's'}
              </span>
              <span class="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-300 font-mono border border-white/5">
                {formatBytes($legacyActiveFolderStore.total_bytes)}
              </span>
              <span class="text-[9px] uppercase tracking-wider px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 font-mono border border-emerald-500/20 flex items-center space-x-1">
                <Zap class="w-2.5 h-2.5" />
                <span>Zero-Stall Mode</span>
              </span>
            </div>

            <p class="text-[11px] text-amber-300/80 font-mono mt-1 truncate" title={$legacyActiveFolderStore.folder_path}>
              {$legacyActiveFolderStore.folder_path}
            </p>

            <p class="text-[11px] text-slate-400 mt-1.5 leading-relaxed">
              Thumbnail previews deferred. .JXL files will be converted back to PNG / JPG directly in parallel without UI stalls.
            </p>
          </div>
        </div>
      </div>

      {#if $legacyFolderProgressStore}
        <div class="mt-3 pt-3 border-t border-white/5 space-y-1.5">
          <div class="flex items-center justify-between text-xs font-mono">
            <span class="text-slate-300 truncate max-w-sm">
              {$legacyFolderProgressStore.current_file}
            </span>
            <span class="text-amber-400 font-bold">
              {Math.round(($legacyFolderProgressStore.completed / Math.max(1, $legacyFolderProgressStore.total)) * 100)}% ({$legacyFolderProgressStore.completed} / {$legacyFolderProgressStore.total})
            </span>
          </div>
          <div class="w-full h-2 bg-slate-900 rounded-full overflow-hidden border border-white/5">
            <div
              class="h-full bg-gradient-to-r from-amber-500 to-orange-500 transition-all duration-150"
              style={`width: ${Math.round(($legacyFolderProgressStore.completed / Math.max(1, $legacyFolderProgressStore.total)) * 100)}%`}
            ></div>
          </div>
        </div>
      {/if}
    </div>
  {/if}

  {#if $legacyItemsStore.length === 0 && !$legacyActiveFolderStore}
    <div class="h-full flex flex-col items-center justify-center text-slate-400 py-12 select-none">
      <div class="w-16 h-16 rounded-2xl bg-surface-850 border border-white/5 flex items-center justify-center mb-3">
        <FileDown class="w-8 h-8 text-amber-500/50" />
      </div>
      <p class="text-sm font-medium text-slate-300">No .jxl files queued for conversion</p>
      <p class="text-xs text-slate-500 mt-1 text-center max-w-xs">
        Drag and drop .jxl files or folders above to convert them back to PNG or JPG
      </p>
    </div>
  {:else}
    {#each $legacyItemsStore as item (item.id)}
      <LegacyQueueItemCard {item} />
    {/each}
  {/if}
</div>
