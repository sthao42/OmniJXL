<script lang="ts">
  import type { ImageItem } from '../types';
  import { formatBytes, removeItem, isProcessingStore } from '../stores/queue';
  import { FileImage, Loader2, X, AlertCircle } from '@lucide/svelte';

  export let item: ImageItem;

  function getBadgeColor(type: string): string {
    const t = type.toUpperCase();
    if (t.includes('PNG')) return 'bg-indigo-950/70 text-indigo-300 border-indigo-700/30';
    if (t.includes('RAW') || t.includes('CR2') || t.includes('NEF') || t.includes('ARW')) {
      return 'bg-amber-950/70 text-amber-300 border-amber-700/30';
    }
    if (t.includes('EXR') || t.includes('HDR')) return 'bg-purple-950/70 text-purple-300 border-purple-700/30';
    if (t.includes('JPEG') || t.includes('JPG')) return 'bg-blue-950/70 text-blue-300 border-blue-700/30';
    if (t.includes('WEBP') || t.includes('AVIF')) return 'bg-teal-950/70 text-teal-300 border-teal-700/30';
    return 'bg-slate-800 text-slate-300 border-white/5';
  }

  $: badgeColor = getBadgeColor(item.format);
  $: savingsRatio =
    item.target_size && item.orig_size
      ? Math.round((1 - item.target_size / item.orig_size) * 100)
      : 0;
</script>

<div class="glass-panel p-3.5 rounded-xl flex items-center justify-between border border-white/5 hover:border-white/10 transition-colors">
  <div class="flex items-center space-x-3 min-w-0">
    <!-- Thumbnail preview -->
    <div class="w-10 h-10 rounded-lg bg-surface-900 border border-white/10 flex items-center justify-center shrink-0 overflow-hidden relative">
      {#if item.thumbnail}
        <img src={item.thumbnail} alt={item.name} class="w-full h-full object-cover" />
      {:else}
        <FileImage class="w-5 h-5 text-slate-400" />
      {/if}
    </div>

    <!-- Metadata details -->
    <div class="min-w-0">
      <div class="flex items-center space-x-2">
        <span class="text-xs font-medium text-slate-200 truncate max-w-xs">{item.name}</span>
        <span class={`text-[10px] px-1.5 py-0.2 rounded font-mono border ${badgeColor}`}>
          {item.format}
        </span>
        {#if item.metadata?.has_comfy_workflow}
          <span class="text-[9px] px-1.5 py-0.2 rounded bg-purple-950/80 text-purple-300 border border-purple-500/40 font-mono font-semibold" title={item.metadata?.comfy_nodes_count ? `${item.metadata.comfy_nodes_count} ComfyUI Nodes` : 'ComfyUI Workflow Embedded'}>
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
        <span>{formatBytes(item.orig_size)}</span>
        <span>•</span>
        <span>{item.width} × {item.height}</span>
        <span>•</span>
        <span class="text-slate-300">JXL Target</span>
      </div>
    </div>
  </div>

  <!-- Status & Removal -->
  <div class="flex items-center space-x-4 pl-3">
    {#if item.status === 'pending'}
      <span class="text-xs text-slate-400 font-mono">Queued</span>
    {:else if item.status === 'converting'}
      <div class="flex items-center space-x-2 text-xs text-jxl-teal font-mono">
        <Loader2 class="w-3.5 h-3.5 animate-spin text-jxl-teal" />
        <span>{item.progress}%</span>
      </div>
    {:else if item.status === 'done'}
      <div class="text-right font-mono">
        <span class="text-xs font-bold text-emerald-400">-{savingsRatio}%</span>
        <div class="text-[10px] text-slate-400">
          {formatBytes(item.target_size || 0)} · {item.time_ms || 0}ms
        </div>
      </div>
    {:else if item.status === 'error'}
      <div class="flex items-center space-x-1 text-red-400 text-xs font-mono" title={item.error_message || 'Conversion failed'}>
        <AlertCircle class="w-3.5 h-3.5" />
        <span>Error</span>
      </div>
    {/if}

    <button
      on:click={() => removeItem(item.id)}
      disabled={$isProcessingStore}
      class="text-slate-400 hover:text-red-400 p-1.5 transition-colors rounded-lg hover:bg-slate-800 disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
      title="Remove item"
    >
      <X class="w-4 h-4" />
    </button>
  </div>
</div>
