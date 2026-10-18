<script lang="ts">
  import type { ImageItem } from '../types';
  import { formatBytes, removeLegacyItem, isLegacyProcessingStore } from '../stores/queue';
  import { FileImage, Loader2, X, AlertCircle, Sparkles } from '@lucide/svelte';

  export let item: ImageItem;
</script>

<div class="glass-panel p-3.5 rounded-xl flex items-center justify-between border border-white/5 hover:border-white/10 transition-colors">
  <div class="flex items-center space-x-3 min-w-0">
    <!-- Thumbnail preview -->
    <div class="w-10 h-10 rounded-lg bg-surface-900 border border-white/10 flex items-center justify-center shrink-0 overflow-hidden relative">
      {#if item.thumbnail}
        <img src={item.thumbnail} alt={item.name} class="w-full h-full object-cover" />
      {:else}
        <FileImage class="w-5 h-5 text-amber-400/70" />
      {/if}
    </div>

    <!-- Metadata details -->
    <div class="min-w-0">
      <div class="flex items-center space-x-2">
        <span class="text-xs font-medium text-slate-200 truncate max-w-xs">{item.name}</span>
        <span class="text-[10px] px-1.5 py-0.2 rounded font-mono border bg-amber-950/70 text-amber-300 border-amber-700/30">
          JXL
        </span>
        {#if item.metadata?.has_comfy_workflow}
          <span
            class="text-[9px] px-1.5 py-0.2 rounded bg-purple-950/80 text-purple-300 border border-purple-500/40 font-mono font-semibold"
            title={item.metadata?.comfy_nodes_count ? `${item.metadata.comfy_nodes_count} ComfyUI Nodes` : 'ComfyUI Workflow Embedded in JXL'}
          >
            ComfyUI {#if item.metadata?.comfy_nodes_count}({item.metadata.comfy_nodes_count}n){/if}
          </span>
        {/if}
        {#if item.metadata?.has_icc}
          <span class="text-[9px] px-1 rounded bg-slate-800 text-slate-400 border border-white/5 font-mono" title="Embedded ICC Color Profile">
            ICC
          </span>
        {/if}
        {#if item.metadata?.has_exif}
          <span class="text-[9px] px-1 rounded bg-slate-800 text-slate-400 border border-white/5 font-mono" title={item.metadata?.date_time || 'EXIF Metadata'}>
            EXIF
          </span>
        {/if}
        {#if item.metadata?.has_xmp}
          <span class="text-[9px] px-1 rounded bg-slate-800 text-slate-400 border border-white/5 font-mono" title="Adobe XMP Data">
            XMP
          </span>
        {/if}
      </div>

      <div class="text-[11px] text-slate-400 flex items-center space-x-2 mt-0.5 font-mono">
        <span class="text-red-400/90 font-medium">{formatBytes(item.orig_size)}</span>
        <span>•</span>
        <span>{item.width} × {item.height}</span>
        <span>•</span>
        <span class="text-amber-400/90">PNG / JPG</span>
      </div>
    </div>
  </div>

  <!-- Status & Removal -->
  <div class="flex items-center space-x-4 pl-3">
    {#if item.status === 'pending'}
      <span class="text-xs text-slate-400 font-mono">Queued</span>
    {:else if item.status === 'converting'}
      <div class="flex items-center space-x-2 text-xs text-amber-400 font-mono">
        <Loader2 class="w-3.5 h-3.5 animate-spin text-amber-400" />
        <span>{item.progress}%</span>
      </div>
    {:else if item.status === 'done'}
      <div class="text-right font-mono">
        <span class="text-xs font-bold text-emerald-400">{formatBytes(item.target_size || 0)}</span>
        <div class="text-[10px] text-slate-400">
          {item.time_ms || 0}ms
        </div>
      </div>
    {:else if item.status === 'error'}
      <div class="flex items-center space-x-1 text-red-400 text-xs font-mono" title={item.error_message || 'Conversion failed'}>
        <AlertCircle class="w-3.5 h-3.5" />
        <span>Error</span>
      </div>
    {/if}

    <button
      on:click={() => removeLegacyItem(item.id)}
      disabled={$isLegacyProcessingStore}
      class="p-1 rounded-lg text-slate-500 hover:text-slate-200 hover:bg-slate-800 transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
      title="Remove from queue"
    >
      <X class="w-4 h-4" />
    </button>
  </div>
</div>
