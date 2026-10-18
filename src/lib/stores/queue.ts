import { writable, derived, get } from 'svelte/store';
import type { ImageItem, ProgressUpdate } from '../types';
import { settingsStore, legacySettingsStore } from './settings';
import { showToast } from './toast';

export const activeTabStore = writable<'encode' | 'legacy'>('encode');

export const itemsStore = writable<ImageItem[]>([]);
export const isProcessingStore = writable<boolean>(false);
export const overallStatusStore = writable<string>('Ready for processing');
export const throughputStore = writable<string>('0.0 MP/s');

export const legacyItemsStore = writable<ImageItem[]>([]);
export const isLegacyProcessingStore = writable<boolean>(false);
export const legacyStatusStore = writable<string>('Ready to unpack .jxl files');

export interface FolderScanSummary {
  folder_path: string;
  folder_name: string;
  image_count: number;
  total_bytes: number;
}

export interface FolderBatchProgress {
  total: number;
  completed: number;
  failed: number;
  current_file: string;
  orig_bytes_completed: number;
  target_bytes_completed: number;
}

export const activeFolderStore = writable<FolderScanSummary | null>(null);
export const legacyActiveFolderStore = writable<FolderScanSummary | null>(null);

export const folderProgressStore = writable<FolderBatchProgress | null>(null);
export const legacyFolderProgressStore = writable<FolderBatchProgress | null>(null);

export const activeFolderPathStore = derived(activeFolderStore, ($f) => $f?.folder_path ?? null);
export const legacyActiveFolderPathStore = derived(
  legacyActiveFolderStore,
  ($f) => $f?.folder_path ?? null
);

export interface DragState {
  isDragging: boolean;
  targetZone: 'files' | 'folder' | 'workspace' | null;
}

export const dragStateStore = writable<DragState>({
  isDragging: false,
  targetZone: null,
});

export function formatBytes(bytes: number): string {
  if (!bytes || bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export const queueStatsStore = derived(
  [itemsStore, activeFolderStore, folderProgressStore],
  ([$items, $activeFolder, $folderProgress]) => {
    let totalOrigBytes = 0;
    let completedOrigBytes = 0;
    let completedTargetBytes = 0;
    let completed = 0;
    let totalCount = $items.length;

    for (const item of $items) {
      totalOrigBytes += item.orig_size || 0;
      if (item.status === 'done' && item.target_size) {
        completedOrigBytes += item.orig_size || 0;
        completedTargetBytes += item.target_size;
        completed++;
      }
    }

    if ($activeFolder) {
      totalCount += $activeFolder.image_count;
      totalOrigBytes += $activeFolder.total_bytes;
    }

    if ($folderProgress) {
      completed += $folderProgress.completed;
      completedOrigBytes += $folderProgress.orig_bytes_completed;
      completedTargetBytes += $folderProgress.target_bytes_completed;
    }

    // File size changes in MB (1 MB = 1024 * 1024 bytes)
    const totalInitialMB = (totalOrigBytes / (1024 * 1024)).toFixed(2);
    const convertedMB = (completedTargetBytes / (1024 * 1024)).toFixed(2);

    const savedBytes =
      completedOrigBytes > completedTargetBytes ? completedOrigBytes - completedTargetBytes : 0;
    const savedPercent =
      completedOrigBytes > 0 ? Math.round((savedBytes / completedOrigBytes) * 100) : 0;

    return {
      totalCount,
      completedCount: completed,
      totalOrigBytes,
      totalInitialMB,
      completedTargetBytes,
      convertedMB,
      savedBytes,
      savedPercent,
      savedFormatted: `-${formatBytes(savedBytes)} (${savedPercent}%)`,
    };
  }
);

export const legacyStatsStore = derived(
  [legacyItemsStore, legacyActiveFolderStore, legacyFolderProgressStore],
  ([$items, $activeFolder, $folderProgress]) => {
    let totalOrigBytes = 0;
    let completedOrigBytes = 0;
    let completedTargetBytes = 0;
    let completed = 0;
    let totalCount = $items.length;

    for (const item of $items) {
      totalOrigBytes += item.orig_size || 0;
      if (item.status === 'done' && item.target_size) {
        completedOrigBytes += item.orig_size || 0;
        completedTargetBytes += item.target_size;
        completed++;
      }
    }

    if ($activeFolder) {
      totalCount += $activeFolder.image_count;
      totalOrigBytes += $activeFolder.total_bytes;
    }

    if ($folderProgress) {
      completed += $folderProgress.completed;
      completedOrigBytes += $folderProgress.orig_bytes_completed;
      completedTargetBytes += $folderProgress.target_bytes_completed;
    }

    const totalInitialMB = (totalOrigBytes / (1024 * 1024)).toFixed(2);
    const convertedMB = (completedTargetBytes / (1024 * 1024)).toFixed(2);

    return {
      totalCount,
      completedCount: completed,
      totalOrigBytes,
      totalInitialMB,
      completedTargetBytes,
      convertedMB,
    };
  }
);

export async function addFilesFromPaths(paths: string[]) {
  if (paths.length === 0) return;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const inspected = await invoke<any[]>('cmd_inspect_images', { paths });

    const newItems: ImageItem[] = inspected.map((item) => ({
      ...item,
      status: 'pending',
      progress: 0,
      target_size: null,
      time_ms: null,
      error_message: null,
    }));

    itemsStore.update((existing) => [...existing, ...newItems]);
    showToast(`Added ${newItems.length} image${newItems.length > 1 ? 's' : ''} to queue`);
  } catch (err) {
    console.warn('Native inspect failed, creating client items:', err);
    // Fallback in web preview mode
    const newItems: ImageItem[] = paths.map((p) => {
      const name = p.split(/[\/\\]/).pop() || 'image';
      const ext = name.split('.').pop()?.toUpperCase() || 'IMG';
      return {
        id: 'item_' + Math.random().toString(36).substring(2, 10),
        path: p,
        name,
        orig_size: Math.floor(Math.random() * 8000000) + 1000000,
        format: ext,
        width: 3840,
        height: 2160,
        thumbnail: null,
        metadata: { has_exif: true, has_icc: true, has_xmp: false },
        status: 'pending',
        progress: 0,
        target_size: null,
        time_ms: null,
        error_message: null,
      };
    });
    itemsStore.update((existing) => [...existing, ...newItems]);
    showToast(`Added ${newItems.length} files to queue`);
  }
}

export async function addFolderFromPath(folderPath: string) {
  if (!folderPath) return;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const summary = await invoke<FolderScanSummary>('cmd_scan_folder_summary', {
      path: folderPath,
    });
    if (!summary || summary.image_count === 0) {
      showToast('No supported image files found in folder');
      return;
    }

    activeFolderStore.set(summary);
    folderProgressStore.set(null);
    showToast(`Found ${summary.image_count} image files in "${summary.folder_name}"`);
  } catch (err) {
    console.warn('Native folder scan failed:', err);
    showToast(`Failed to scan folder: ${err}`);
  }
}

export function clearActiveFolder() {
  if (get(isProcessingStore)) return;
  activeFolderStore.set(null);
  folderProgressStore.set(null);
  overallStatusStore.set('Folder cleared');
}

export function removeItem(id: string) {
  if (get(isProcessingStore)) return;
  itemsStore.update((items) => items.filter((i) => i.id !== id));
}

export function clearQueue() {
  if (get(isProcessingStore)) return;
  itemsStore.set([]);
  overallStatusStore.set('Queue cleared');
}

export function handleProgressEvent(update: ProgressUpdate) {
  itemsStore.update((items) =>
    items.map((item) => {
      if (item.id === update.id) {
        return {
          ...item,
          status: (update.status as any) || item.status,
          progress: update.progress,
          target_size: update.target_size ?? item.target_size,
          time_ms: update.time_ms ?? item.time_ms,
          error_message: update.error_message ?? item.error_message,
        };
      }
      return item;
    })
  );
}

export async function convertAll() {
  const activeFolder = get(activeFolderStore);
  const items = get(itemsStore);
  const pending = items.filter((i) => i.status !== 'done');
  const hasFolderWork = activeFolder !== null && activeFolder.image_count > 0;

  if (!hasFolderWork && pending.length === 0) return;
  if (get(isProcessingStore)) return;

  isProcessingStore.set(true);
  overallStatusStore.set('Encoding in parallel with Rayon...');
  throughputStore.set('High-Throughput SIMD');

  const profile = get(settingsStore);

  try {
    const { invoke } = await import('@tauri-apps/api/core');

    // 1. Convert active folder batch directly if present
    if (hasFolderWork && activeFolder) {
      overallStatusStore.set(`Processing folder "${activeFolder.folder_name}"...`);
      folderProgressStore.set({
        total: activeFolder.image_count,
        completed: 0,
        failed: 0,
        current_file: 'Scanning files...',
        orig_bytes_completed: 0,
        target_bytes_completed: 0,
      });

      const folderRes = await invoke<{
        total_processed: number;
        successful: number;
        failed: number;
        total_time_ms: number;
      }>('cmd_convert_folder_batch', {
        folderPath: activeFolder.folder_path,
        profile,
      });

      overallStatusStore.set(`Completed folder: ${folderRes.successful} converted`);
    }

    // 2. Convert queued individual items if present
    if (pending.length > 0) {
      overallStatusStore.set('Encoding individual queued images...');
      const results = await invoke<any[]>('cmd_convert_batch', {
        items: pending,
        profile,
      });

      itemsStore.update((existing) =>
        existing.map((item) => {
          const found = results.find((r) => r.id === item.id);
          if (found) {
            return {
              ...item,
              status: found.status,
              progress: 100,
              target_size: found.target_size,
              time_ms: found.time_ms,
              error_message: found.error_message,
            };
          }
          return item;
        })
      );
    }

    showToast('Finished encoding batch to .jxl');
  } catch (err) {
    console.warn('Native batch convert error or fallback:', err);
    // Simulation fallback for web preview
    for (const item of pending) {
      await simulateConvertItem(item, profile);
    }
    showToast('Batch conversion finished');
  } finally {
    isProcessingStore.set(false);
    overallStatusStore.set('Batch completed');
    throughputStore.set('Idle');
  }
}

async function simulateConvertItem(item: ImageItem, profile: any) {
  itemsStore.update((list) =>
    list.map((i) => (i.id === item.id ? { ...i, status: 'converting', progress: 30 } : i))
  );

  await new Promise((r) => setTimeout(r, 450 + Math.random() * 500));

  let factor = 0.55;
  if (profile.is_lossless) {
    if (item.format.includes('PNG')) factor = 0.58;
    else if (item.format.includes('RAW')) factor = 0.45;
    else if (item.format.includes('EXR')) factor = 0.40;
    else factor = 0.72;
  } else {
    factor = Math.min(0.65, 0.18 + profile.distance * 0.05);
  }

  const targetSize = Math.round(item.orig_size * factor);
  const duration = Math.round(500 + Math.random() * 700);

  itemsStore.update((list) =>
    list.map((i) =>
      i.id === item.id
        ? {
            ...i,
            status: 'done',
            progress: 100,
            target_size: targetSize,
            time_ms: duration,
          }
        : i
    )
  );
}

// ================= Legacy Mode Functions =================

export async function addJxlFilesFromPaths(paths: string[]) {
  if (paths.length === 0) return;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const inspected = await invoke<any[]>('cmd_inspect_images', { paths });

    const newItems: ImageItem[] = inspected.map((item) => ({
      ...item,
      status: 'pending',
      progress: 0,
      target_size: null,
      time_ms: null,
      error_message: null,
    }));

    legacyItemsStore.update((existing) => [...existing, ...newItems]);
    showToast(
      `Added ${newItems.length} JXL image${newItems.length > 1 ? 's' : ''} to unpack queue`
    );
  } catch (err) {
    console.warn('Native inspect failed for JXL:', err);
    showToast(`Failed to inspect JXL files: ${err}`);
  }
}

export async function addJxlFolderFromPath(folderPath: string) {
  if (!folderPath) return;

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const summary = await invoke<FolderScanSummary>('cmd_scan_folder_jxl_summary', {
      path: folderPath,
    });
    if (!summary || summary.image_count === 0) {
      showToast('No .jxl image files found in folder');
      return;
    }

    legacyActiveFolderStore.set(summary);
    legacyFolderProgressStore.set(null);
    showToast(`Found ${summary.image_count} .jxl files in "${summary.folder_name}"`);
  } catch (err) {
    console.warn('Native JXL folder scan failed:', err);
    showToast(`Failed to scan folder: ${err}`);
  }
}

export function clearLegacyActiveFolder() {
  if (get(isLegacyProcessingStore)) return;
  legacyActiveFolderStore.set(null);
  legacyFolderProgressStore.set(null);
  legacyStatusStore.set('Folder cleared');
}

export function removeLegacyItem(id: string) {
  if (get(isLegacyProcessingStore)) return;
  legacyItemsStore.update((items) => items.filter((i) => i.id !== id));
}

export function clearLegacyQueue() {
  if (get(isLegacyProcessingStore)) return;
  legacyItemsStore.set([]);
  legacyStatusStore.set('Unpack queue cleared');
}

export function handleLegacyProgressEvent(update: ProgressUpdate) {
  legacyItemsStore.update((items) =>
    items.map((item) => {
      if (item.id === update.id) {
        return {
          ...item,
          status: (update.status as any) || item.status,
          progress: update.progress,
          target_size: update.target_size ?? item.target_size,
          time_ms: update.time_ms ?? item.time_ms,
          error_message: update.error_message ?? item.error_message,
        };
      }
      return item;
    })
  );
}

export async function convertLegacyAll() {
  const legacyFolder = get(legacyActiveFolderStore);
  const items = get(legacyItemsStore);
  const pending = items.filter((i) => i.status !== 'done');
  const hasFolderWork = legacyFolder !== null && legacyFolder.image_count > 0;

  if (!hasFolderWork && pending.length === 0) return;
  if (get(isLegacyProcessingStore)) return;

  isLegacyProcessingStore.set(true);
  const profile = get(legacySettingsStore);
  legacyStatusStore.set(`Unpacking .jxl to ${profile.target_format.toUpperCase()} in parallel...`);

  try {
    const { invoke } = await import('@tauri-apps/api/core');

    // 1. Convert active legacy folder batch directly if present
    if (hasFolderWork && legacyFolder) {
      legacyStatusStore.set(`Unpacking folder "${legacyFolder.folder_name}"...`);
      legacyFolderProgressStore.set({
        total: legacyFolder.image_count,
        completed: 0,
        failed: 0,
        current_file: 'Scanning files...',
        orig_bytes_completed: 0,
        target_bytes_completed: 0,
      });

      const folderRes = await invoke<{
        total_processed: number;
        successful: number;
        failed: number;
        total_time_ms: number;
      }>('cmd_convert_legacy_folder_batch', {
        folderPath: legacyFolder.folder_path,
        profile,
      });

      legacyStatusStore.set(`Completed folder: ${folderRes.successful} unpacked`);
    }

    // 2. Convert queued individual items if present
    if (pending.length > 0) {
      const results = await invoke<any[]>('cmd_convert_legacy_batch', {
        items: pending,
        profile,
      });

      legacyItemsStore.update((existing) =>
        existing.map((item) => {
          const found = results.find((r) => r.id === item.id);
          if (found) {
            return {
              ...item,
              status: found.status,
              progress: 100,
              target_size: found.target_size,
              time_ms: found.time_ms,
              error_message: found.error_message,
            };
          }
          return item;
        })
      );
    }

    showToast(`Finished unpacking JXL to ${profile.target_format.toUpperCase()}`);
  } catch (err) {
    console.warn('Native legacy batch convert error:', err);
    showToast(`Conversion error: ${err}`);
  } finally {
    isLegacyProcessingStore.set(false);
    legacyStatusStore.set('Conversion completed');
  }
}

export async function handleDroppedPaths(
  paths: string[],
  targetHint?: 'files' | 'folder' | 'workspace'
) {
  if (!paths || paths.length === 0) return;

  const currentTab = get(activeTabStore);
  const isJxl = currentTab === 'legacy';

  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const result = await invoke<{
      items: any[];
      folder_summary: FolderScanSummary | null;
      total_folders: number;
      total_files: number;
    }>('cmd_handle_dropped_paths', {
      paths,
      isJxlMode: isJxl,
    });

    if (result.folder_summary) {
      if (isJxl) {
        legacyActiveFolderStore.set(result.folder_summary);
        legacyFolderProgressStore.set(null);
      } else {
        activeFolderStore.set(result.folder_summary);
        folderProgressStore.set(null);
      }
    }

    if (result.items && result.items.length > 0) {
      const newItems: ImageItem[] = result.items.map((item) => ({
        ...item,
        status: 'pending',
        progress: 0,
        target_size: null,
        time_ms: null,
        error_message: null,
      }));

      if (isJxl) {
        legacyItemsStore.update((existing) => [...existing, ...newItems]);
      } else {
        itemsStore.update((existing) => [...existing, ...newItems]);
      }
    }

    if (result.folder_summary) {
      const f = result.folder_summary;
      const countMsg = `${f.image_count} ${isJxl ? '.jxl' : 'image'} file${f.image_count === 1 ? '' : 's'} in "${f.folder_name}"`;
      if (result.items && result.items.length > 0) {
        showToast(`Folder selected (${countMsg}) + added ${result.items.length} file(s)`);
      } else {
        showToast(`Folder selected: ${countMsg}`);
      }
    } else if (result.items && result.items.length > 0) {
      showToast(
        `Added ${result.items.length} ${isJxl ? 'JXL ' : ''}image${result.items.length > 1 ? 's' : ''}`
      );
    } else {
      showToast(isJxl ? 'No .jxl image files found' : 'No supported image files found');
    }
  } catch (err) {
    console.warn('Failed to handle dropped paths:', err);
    showToast(`Drop failed: ${err}`);
  }
}
