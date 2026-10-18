<script lang="ts">
  import { systemStore } from '../stores/system';
  import {
    clearQueue,
    clearLegacyQueue,
    isProcessingStore,
    isLegacyProcessingStore,
    activeTabStore,
    itemsStore,
    legacyItemsStore,
  } from '../stores/queue';
  import { Trash2, Sparkles, History } from '@lucide/svelte';

  function handleClear() {
    if ($activeTabStore === 'encode') {
      clearQueue();
    } else {
      clearLegacyQueue();
    }
  }

  $: isCurrentProcessing =
    $activeTabStore === 'encode' ? $isProcessingStore : $isLegacyProcessingStore;
  $: currentItemCount =
    $activeTabStore === 'encode' ? $itemsStore.length : $legacyItemsStore.length;
</script>

<header class="h-11 bg-surface-850 border-b border-white/5 flex items-center justify-between px-4 shrink-0 z-50 select-none gap-4">
  <div class="flex items-center space-x-3 shrink-0">
    <!-- App Identity -->
    <div class="flex items-center space-x-2">
      <img
        src="/app-icon.png"
        alt="OmniJXL Logo"
        class="w-5 h-5 rounded shadow-sm shadow-jxl-teal/20 object-contain"
      />
      <span class="text-xs font-semibold tracking-wider text-slate-200">OmniJXL</span>
      <span class="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-emerald-400 font-mono border border-emerald-500/20">
        {$systemStore.libjxl_version} · Rayon x{$systemStore.threads}
      </span>
    </div>
  </div>

  <!-- Center Tab Selector Navigation -->
  <div class="flex items-center bg-surface-900/90 p-1 rounded-xl border border-white/10 gap-1 text-xs font-medium">
    <button
      type="button"
      on:click={() => activeTabStore.set('encode')}
      class={`px-3.5 py-1 rounded-lg transition-all flex items-center space-x-1.5 cursor-pointer ${
        $activeTabStore === 'encode'
          ? 'bg-brand-600 text-white shadow-sm font-semibold'
          : 'text-slate-400 hover:text-slate-200'
      }`}
    >
      <Sparkles class="w-3.5 h-3.5 text-jxl-teal" />
      <span>Encode to .JXL</span>
      {#if $itemsStore.length > 0}
        <span class="text-[10px] px-1.5 py-0.2 rounded-full bg-slate-800 text-slate-300 font-mono ml-1">
          {$itemsStore.length}
        </span>
      {/if}
    </button>

    <button
      type="button"
      on:click={() => activeTabStore.set('legacy')}
      class={`px-3.5 py-1 rounded-lg transition-all flex items-center space-x-1.5 cursor-pointer ${
        $activeTabStore === 'legacy'
          ? 'bg-brand-600 text-white shadow-sm font-semibold'
          : 'text-slate-400 hover:text-slate-200'
      }`}
    >
      <History class="w-3.5 h-3.5 text-amber-400" />
      <span>JXL to PNG / JPG</span>
      {#if $legacyItemsStore.length > 0}
        <span class="text-[10px] px-1.5 py-0.2 rounded-full bg-slate-800 text-slate-300 font-mono ml-1">
          {$legacyItemsStore.length}
        </span>
      {/if}
    </button>
  </div>

  <!-- Action Tools -->
  <div class="flex items-center space-x-2.5 text-xs shrink-0">
    <button
      on:click={handleClear}
      disabled={isCurrentProcessing || currentItemCount === 0}
      class="px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors flex items-center space-x-1.5 disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
      title="Clear queued items in current tab"
    >
      <Trash2 class="w-3.5 h-3.5" />
      <span>Clear</span>
    </button>
  </div>
</header>
