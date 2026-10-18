import { writable } from 'svelte/store';
import type { EncodingProfile, LegacyExportProfile } from '../types';

export const settingsStore = writable<EncodingProfile>({
  is_lossless: true,
  distance: 1.0,
  effort: 7,
  is_progressive: true,
  preserve_metadata: true,
  reversible_jpeg: true,
  dest_folder: '~/Pictures/JXL_Exports',
});

export const legacySettingsStore = writable<LegacyExportProfile>({
  target_format: 'png',
  jpeg_quality: 95,
  reconstruct_jpeg: true,
  dest_folder: '~/Pictures/JXL_Unpack',
});
