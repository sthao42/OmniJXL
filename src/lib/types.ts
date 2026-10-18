export interface ImageMetadataSummary {
  make?: string | null;
  model?: string | null;
  date_time?: string | null;
  iso?: number | null;
  exposure_time?: string | null;
  f_number?: string | null;
  focal_length?: string | null;
  gps_latitude?: string | null;
  gps_longitude?: string | null;
  has_icc: boolean;
  has_exif: boolean;
  has_xmp: boolean;
  has_comfy_workflow?: boolean;
  comfy_nodes_count?: number | null;
}

export interface ImageItem {
  id: string;
  path: string;
  name: string;
  orig_size: number;
  format: string;
  width: number;
  height: number;
  thumbnail?: string | null;
  metadata?: ImageMetadataSummary;
  status: 'pending' | 'converting' | 'done' | 'error';
  progress: number;
  target_size?: number | null;
  time_ms?: number | null;
  error_message?: string | null;
}

export interface EncodingProfile {
  is_lossless: boolean;
  distance: number;
  effort: number;
  is_progressive: boolean;
  preserve_metadata: boolean;
  reversible_jpeg: boolean;
  dest_folder?: string | null;
}

export interface SystemInfo {
  threads: number;
  simd: string;
  libjxl_version: string;
  cjxl_path?: string | null;
  djxl_path?: string | null;
  is_ready: boolean;
}

export interface LegacyExportProfile {
  target_format: 'png' | 'jpg';
  jpeg_quality: number;
  reconstruct_jpeg: boolean;
  dest_folder?: string | null;
}

export interface ProgressUpdate {
  id: string;
  status: string;
  progress: number;
  target_size?: number | null;
  savings_bytes?: number | null;
  savings_percent?: number | null;
  time_ms?: number | null;
  error_message?: string | null;
}
