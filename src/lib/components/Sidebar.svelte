<script lang="ts">
  import { settingsStore } from '../stores/settings';
  import { systemStore } from '../stores/system';
  import { showToast } from '../stores/toast';
  import {
    Sliders,
    ShieldCheck,
    Eye,
    FolderOpen,
    ExternalLink,
    HardDrive,
  } from '@lucide/svelte';

  const effortLabels: Record<number, string> = {
    1: '1 (Lightning)',
    2: '2 (Fast)',
    3: '3 (Falcon)',
    4: '4 (Cheetah)',
    5: '5 (Default)',
    6: '6 (Wombat)',
    7: '7 (Tortoise)',
    8: '8 (Kitten)',
    9: '9 (Glacier/Max)',
  };

  async function handleBrowseFolder() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selected = await invoke<string | null>('cmd_select_folder');
      if (selected) {
        settingsStore.update((s) => ({ ...s, dest_folder: selected }));
        showToast(`Destination folder set to: ${selected}`);
      }
    } catch {
      showToast('Selected default output: ~/Pictures/JXL_Exports');
    }
  }

  async function handleOpenFolder() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cmd_open_folder', { path: $settingsStore.dest_folder });
    } catch {
      showToast('Opened destination directory in file explorer');
    }
  }

  function setMode(lossless: boolean) {
    settingsStore.update((s) => ({ ...s, is_lossless: lossless }));
  }
</script>

<aside class="w-80 lg:w-96 glass-panel border-l border-white/5 flex flex-col shrink-0 overflow-y-auto select-none">
  <!-- Settings Header -->
  <div class="p-4 border-b border-white/5 flex items-center justify-between">
    <div class="flex items-center space-x-2">
      <Sliders class="w-4 h-4 text-jxl-teal" />
      <h3 class="text-sm font-semibold text-slate-100">Encoding Profile</h3>
    </div>
    <span class="text-[10px] text-slate-400 font-mono px-2 py-0.5 rounded bg-slate-800">
      libjxl SIMD
    </span>
  </div>

  <!-- Settings Content -->
  <div class="p-4 space-y-5 text-xs flex-1">
    <!-- Compression Mode Switch -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <span class="text-slate-300 font-medium">Compression Mode</span>
        <span
          class={`text-[10px] font-mono uppercase font-semibold ${
            $settingsStore.is_lossless ? 'text-emerald-400' : 'text-jxl-teal'
          }`}
        >
          {$settingsStore.is_lossless ? 'Modular Lossless' : 'VarDCT Perceptual'}
        </span>
      </div>

      <div class="grid grid-cols-2 p-1 bg-surface-900/90 rounded-xl border border-white/10 gap-1">
        <button
          type="button"
          on:click={() => setMode(true)}
          class={`py-2 rounded-lg font-semibold transition-all text-center flex items-center justify-center space-x-1.5 cursor-pointer ${
            $settingsStore.is_lossless
              ? 'bg-brand-600 text-white shadow-sm'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          <ShieldCheck class="w-3.5 h-3.5" />
          <span>Lossless</span>
        </button>

        <button
          type="button"
          on:click={() => setMode(false)}
          class={`py-2 rounded-lg font-semibold transition-all text-center flex items-center justify-center space-x-1.5 cursor-pointer ${
            !$settingsStore.is_lossless
              ? 'bg-brand-600 text-white shadow-sm'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          <Eye class="w-3.5 h-3.5" />
          <span>VarDCT Visual</span>
        </button>
      </div>

      <p class="text-[11px] text-slate-400 leading-relaxed">
        {#if $settingsStore.is_lossless}
          Bit-for-bit exact reconstruction. Ideal for PNG replacement, art graphics, and master photography archives.
        {:else}
          Perceptually lossless variable-block DCT encoding. Exceeds AVIF and WebP at high and medium visual fidelity.
        {/if}
      </p>
    </div>

    <!-- Butteraugli Distance Slider (Lossy only) -->
    <div
      class={`space-y-2 transition-opacity duration-200 ${
        $settingsStore.is_lossless ? 'opacity-40 pointer-events-none' : 'opacity-100'
      }`}
    >
      <div class="flex justify-between items-center">
        <span class="text-slate-300 font-medium">Butteraugli Distance</span>
        <div class="flex items-center space-x-1">
          <span class="font-mono text-jxl-teal font-bold">
            {$settingsStore.distance.toFixed(1)}
          </span>
          <span class="text-[10px] text-slate-400">
            {#if $settingsStore.distance <= 0.5}
              (Near-Lossless)
            {:else if $settingsStore.distance <= 1.5}
              (Visually Lossless)
            {:else}
              (Web Optimized)
            {/if}
          </span>
        </div>
      </div>

      <input
        type="range"
        min="0.1"
        max="15.0"
        step="0.1"
        bind:value={$settingsStore.distance}
        class="w-full accent-jxl-teal h-1.5 bg-slate-800 rounded-lg cursor-pointer"
      />
      <div class="flex justify-between text-[10px] text-slate-400 font-mono">
        <span>0.1 (Near-Lossless)</span>
        <span>1.0 (Standard)</span>
        <span>3.0 (Web)</span>
      </div>
    </div>

    <!-- Encoding Effort Slider -->
    <div class="space-y-2">
      <div class="flex justify-between items-center">
        <span class="text-slate-300 font-medium">Encoding Effort</span>
        <span class="font-mono text-jxl-teal font-bold">
          {effortLabels[$settingsStore.effort] || $settingsStore.effort}
        </span>
      </div>
      <input
        type="range"
        min="1"
        max="9"
        step="1"
        bind:value={$settingsStore.effort}
        class="w-full accent-jxl-teal h-1.5 bg-slate-800 rounded-lg cursor-pointer"
      />
      <div class="flex justify-between text-[10px] text-slate-400 font-mono">
        <span>1 (Fast)</span>
        <span>5 (Default)</span>
        <span>7 (High)</span>
        <span>9 (Max)</span>
      </div>
      <p class="text-[11px] text-slate-400 leading-relaxed">
        Higher effort engages deeper decision-tree heuristics and adaptive predictors. Effort 7 matches high-end production workflows.
      </p>
    </div>

    <div class="h-px bg-white/5 my-1"></div>

    <!-- Container Switches -->
    <div class="space-y-3">
      <h4 class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">
        Container Features
      </h4>

      <!-- Progressive Pass -->
      <label class="flex items-center justify-between cursor-pointer group">
        <div class="pr-2">
          <div class="text-slate-200 font-medium">Progressive Decoding</div>
          <div class="text-[11px] text-slate-400">Enables responsive blur-to-sharp streaming</div>
        </div>
        <input
          type="checkbox"
          bind:checked={$settingsStore.is_progressive}
          class="w-4 h-4 accent-jxl-teal rounded bg-slate-800 border-white/10 cursor-pointer"
        />
      </label>

      <!-- Preserve Metadata -->
      <label class="flex items-center justify-between cursor-pointer group">
        <div class="pr-2">
          <div class="text-slate-200 font-medium">Copy EXIF & ICC Color Profiles</div>
          <div class="text-[11px] text-slate-400">Maintains camera metadata and wide-gamuts</div>
        </div>
        <input
          type="checkbox"
          bind:checked={$settingsStore.preserve_metadata}
          class="w-4 h-4 accent-jxl-teal rounded bg-slate-800 border-white/10 cursor-pointer"
        />
      </label>

      <!-- Reversible JPEG Transcode -->
      <label class="flex items-center justify-between cursor-pointer group">
        <div class="pr-2">
          <div class="text-slate-200 font-medium">Reversible JPEG Transcode</div>
          <div class="text-[11px] text-slate-400">Zero recompression generation loss on existing .jpg</div>
        </div>
        <input
          type="checkbox"
          bind:checked={$settingsStore.reversible_jpeg}
          class="w-4 h-4 accent-jxl-teal rounded bg-slate-800 border-white/10 cursor-pointer"
        />
      </label>
    </div>

    <!-- Engine Architecture Spec Card -->
    <div class="p-3 bg-surface-900/80 rounded-xl border border-white/5 space-y-1.5 font-mono text-[11px]">
      <div class="text-slate-400 uppercase text-[9px] font-sans font-bold tracking-wider">
        Engine Architecture
      </div>
      <div class="flex justify-between text-slate-300">
        <span>Runtime:</span>
        <span class="text-jxl-teal">Tauri v2 + Rust 1.99.1</span>
      </div>
      <div class="flex justify-between text-slate-300">
        <span>Scheduler:</span>
        <span>rayon::par_iter()</span>
      </div>
      <div class="flex justify-between text-slate-300">
        <span>Native Codec:</span>
        <span class="truncate max-w-[140px]" title={$systemStore.libjxl_version}>
          {$systemStore.libjxl_version}
        </span>
      </div>
    </div>
  </div>

  <!-- Destination Folder Selector -->
  <div class="p-4 bg-surface-850/80 border-t border-white/5 text-xs space-y-2">
    <div class="flex items-center justify-between">
      <label for="dest-folder-input" class="text-slate-400 font-medium block">Destination Folder</label>
      <button
        on:click={handleOpenFolder}
        class="text-jxl-teal hover:text-teal-300 text-[10px] flex items-center space-x-1 cursor-pointer"
        title="Open destination folder in Explorer"
      >
        <FolderOpen class="w-3 h-3" />
        <span>Open</span>
      </button>
    </div>
    <div class="flex items-center space-x-2">
      <input
        id="dest-folder-input"
        type="text"
        bind:value={$settingsStore.dest_folder}
        class="flex-1 bg-surface-900 border border-white/10 rounded-lg px-2.5 py-1.5 text-slate-300 font-mono text-[11px] focus:outline-none focus:border-jxl-teal/50"
      />
      <button
        on:click={handleBrowseFolder}
        class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs transition-colors cursor-pointer"
      >
        Browse
      </button>
    </div>
  </div>
</aside>
