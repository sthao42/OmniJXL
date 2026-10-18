import { writable } from 'svelte/store';
import type { SystemInfo } from '../types';

export const systemStore = writable<SystemInfo>({
  threads: 16,
  simd: 'SIMD AVX2 active',
  libjxl_version: 'libjxl 0.12 (SIMD)',
  cjxl_path: null,
  is_ready: true,
});

export async function fetchSystemInfo() {
  try {
    const { invoke } = await import('@tauri-apps/api/core');
    const info = await invoke<SystemInfo>('cmd_get_system_info');
    systemStore.set(info);
    return info;
  } catch (e) {
    console.warn('Tauri IPC not available, running in preview mode:', e);
    return null;
  }
}
