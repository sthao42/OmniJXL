<script lang="ts">
  import { legacySettingsStore } from '../stores/settings';
  import { systemStore } from '../stores/system';
  import { showToast } from '../stores/toast';
  import {
    Sliders,
    FolderOpen,
    ExternalLink,
    HardDrive,
    Sparkles,
    Image as ImageIcon,
    CheckCircle2,
    ShieldCheck,
  } from '@lucide/svelte';

  async function handleBrowseFolder() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const selected = await invoke<string | null>('cmd_select_folder');
      if (selected) {
        legacySettingsStore.update((s) => ({ ...s, dest_folder: selected }));
        showToast(`Destination set to: ${selected}`);
      }
    } catch {
      showToast('Selected default output: ~/Pictures/JXL_Unpack');
    }
  }

  async function handleOpenFolder() {
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('cmd_open_folder', { path: $legacySettingsStore.dest_folder });
    } catch {
      showToast('Opened destination directory in file explorer');
    }
  }

  function setFormat(fmt: 'png' | 'jpg') {
    legacySettingsStore.update((s) => ({ ...s, target_format: fmt }));
  }
</script>

<aside class="w-80 lg:w-96 glass-panel border-l border-white/5 flex flex-col shrink-0 overflow-y-auto select-none">
  <!-- Settings Header -->
  <div class="p-4 border-b border-white/5 flex items-center justify-between">
    <div class="flex items-center space-x-2">
      <Sliders class="w-4 h-4 text-amber-400" />
      <h3 class="text-sm font-semibold text-slate-100">Export Profile</h3>
    </div>
    <span class="text-[10px] text-amber-300 font-mono px-2 py-0.5 rounded bg-amber-500/10 border border-amber-500/20">
      djxl Decoder
    </span>
  </div>

  <!-- Settings Content -->
  <div class="p-4 space-y-5 text-xs flex-1">
    <!-- Format Selector Switch -->
    <div class="space-y-2">
      <div class="flex items-center justify-between">
        <span class="text-slate-300 font-medium">Output Format</span>
        <span class="text-[10px] font-mono uppercase font-semibold text-amber-400">
          {$legacySettingsStore.target_format === 'png' ? 'PNG (Lossless & Workflow)' : 'JPEG (Compressed)'}
        </span>
      </div>

      <div class="grid grid-cols-2 p-1 bg-surface-900/90 rounded-xl border border-white/10 gap-1">
        <button
          type="button"
          on:click={() => setFormat('png')}
          class={`py-2 rounded-lg font-semibold transition-all text-center flex items-center justify-center space-x-1.5 cursor-pointer ${
            $legacySettingsStore.target_format === 'png'
              ? 'bg-amber-600 text-white shadow-sm'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          <ImageIcon class="w-3.5 h-3.5" />
          <span>PNG Format</span>
        </button>

        <button
          type="button"
          on:click={() => setFormat('jpg')}
          class={`py-2 rounded-lg font-semibold transition-all text-center flex items-center justify-center space-x-1.5 cursor-pointer ${
            $legacySettingsStore.target_format === 'jpg'
              ? 'bg-amber-600 text-white shadow-sm'
              : 'text-slate-400 hover:text-white'
          }`}
        >
          <ImageIcon class="w-3.5 h-3.5" />
          <span>JPG Format</span>
        </button>
      </div>
    </div>

    <!-- PNG Features & ComfyUI Workflow Info -->
    {#if $legacySettingsStore.target_format === 'png'}
      <div class="p-3 rounded-xl bg-purple-950/20 border border-purple-500/20 space-y-2">
        <div class="flex items-center space-x-2 text-purple-300 font-semibold">
          <Sparkles class="w-4 h-4 text-purple-400" />
          <span>ComfyUI Workflow Recovery</span>
        </div>
        <p class="text-[11px] text-slate-300 leading-relaxed">
          When unpacking to PNG, embedded ComfyUI workflows, node graphs, and prompt parameters inside JXL container boxes are restored directly into PNG <code class="text-purple-300 font-mono">tEXt</code> chunks.
        </p>
        <div class="flex items-center space-x-2 text-[10px] text-emerald-400 font-mono pt-1">
          <CheckCircle2 class="w-3 h-3 text-emerald-400" />
          <span>Ready for drag-and-drop into ComfyUI</span>
        </div>
      </div>
    {:else}
      <!-- JPEG Specific Options -->
      <div class="space-y-4 p-3 rounded-xl bg-surface-900/60 border border-white/5">
        <!-- JPEG Quality Slider -->
        <div class="space-y-2">
          <div class="flex items-center justify-between">
            <span class="text-slate-300 font-medium">JPEG Quality</span>
            <span class="font-mono text-amber-400 font-semibold">
              {$legacySettingsStore.jpeg_quality}%
            </span>
          </div>
          <input
            type="range"
            min="60"
            max="100"
            step="1"
            bind:value={$legacySettingsStore.jpeg_quality}
            class="w-full accent-amber-500 h-1.5 bg-surface-850 rounded-lg cursor-pointer"
          />
        </div>

        <!-- Lossless JPEG Reconstruction Toggle -->
        <div class="flex items-center justify-between pt-1">
          <div>
            <div class="text-slate-300 font-medium">Reconstruct Original JPEG</div>
            <div class="text-[10px] text-slate-400">
              Bit-for-bit lossless restore if JXL was a JPEG transcode
            </div>
          </div>
          <input
            type="checkbox"
            bind:checked={$legacySettingsStore.reconstruct_jpeg}
            class="w-4 h-4 accent-amber-500 rounded cursor-pointer"
          />
        </div>
      </div>
    {/if}

    <!-- Destination Folder Settings -->
    <div class="space-y-2 pt-2 border-t border-white/5">
      <div class="flex items-center justify-between">
        <span class="text-slate-300 font-medium flex items-center space-x-1.5">
          <HardDrive class="w-3.5 h-3.5 text-slate-400" />
          <span>Output Destination</span>
        </span>
      </div>

      <div class="space-y-2">
        <div class="flex items-center space-x-2">
          <input
            type="text"
            bind:value={$legacySettingsStore.dest_folder}
            placeholder="Default: ~/Pictures/JXL_Unpack"
            class="flex-1 bg-surface-900/90 border border-white/10 rounded-lg px-2.5 py-1.5 text-slate-200 text-xs font-mono focus:outline-none focus:border-amber-400/50"
          />
          <button
            type="button"
            on:click={handleBrowseFolder}
            class="px-2.5 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-200 transition-colors flex items-center space-x-1 cursor-pointer"
            title="Browse Destination Folder"
          >
            <FolderOpen class="w-3.5 h-3.5" />
            <span>Browse</span>
          </button>
        </div>

        <button
          type="button"
          on:click={handleOpenFolder}
          class="w-full py-1.5 rounded-lg bg-surface-850 hover:bg-surface-800 border border-white/5 text-slate-400 hover:text-slate-200 transition-colors flex items-center justify-center space-x-1.5 cursor-pointer text-[11px]"
        >
          <ExternalLink class="w-3.5 h-3.5" />
          <span>Open Destination in File Explorer</span>
        </button>
      </div>
    </div>

    <!-- Preservation Checklist Card -->
    <div class="p-3 rounded-xl bg-surface-900/40 border border-white/5 space-y-2 text-[11px]">
      <span class="font-medium text-slate-300">Preservation Guarantees</span>
      <div class="space-y-1 text-slate-400">
        <div class="flex items-center space-x-1.5">
          <CheckCircle2 class="w-3.5 h-3.5 text-emerald-400 shrink-0" />
          <span>Alpha & Transparency Channels</span>
        </div>
        <div class="flex items-center space-x-1.5">
          <CheckCircle2 class="w-3.5 h-3.5 text-emerald-400 shrink-0" />
          <span>Color Profiles & ICC / EXIF Metadata</span>
        </div>
        <div class="flex items-center space-x-1.5">
          <CheckCircle2 class="w-3.5 h-3.5 text-emerald-400 shrink-0" />
          <span>Parallel Decoding via Rayon Engine</span>
        </div>
      </div>
    </div>
  </div>
</aside>
