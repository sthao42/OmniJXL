<script lang="ts">
  import { toastStore } from '../stores/toast';
  import { CheckCircle2, AlertCircle, Info } from '@lucide/svelte';
</script>

<div
  class={`fixed bottom-6 right-6 px-4 py-3 rounded-xl bg-slate-800 text-white text-xs border border-white/10 shadow-2xl transition-all duration-300 transform flex items-center space-x-2 z-50 pointer-events-none ${
    $toastStore.visible
      ? 'translate-y-0 opacity-100'
      : 'translate-y-16 opacity-0'
  }`}
>
  {#if $toastStore.type === 'error'}
    <AlertCircle class="w-4 h-4 text-red-400 shrink-0" />
  {:else if $toastStore.type === 'info'}
    <Info class="w-4 h-4 text-blue-400 shrink-0" />
  {:else}
    <CheckCircle2 class="w-4 h-4 text-emerald-400 shrink-0" />
  {/if}
  <span class="font-medium text-slate-200">{$toastStore.message}</span>
</div>
