use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::{Path, PathBuf};

use super::metadata::{ImageMetadataSummary, extract_metadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageItemInfo {
    pub id: String,
    pub path: String,
    pub name: String,
    pub orig_size: u64,
    pub format: String,
    pub width: u32,
    pub height: u32,
    pub thumbnail: Option<String>,
    pub metadata: ImageMetadataSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    NativeCjxl,
    Extended,
    Raw,
    Unsupported,
}

pub fn detect_format_type(path: &Path) -> (FormatType, String) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "jxl" => (FormatType::NativeCjxl, "JPEG XL".to_string()),
        "jpg" | "jpeg" => (FormatType::NativeCjxl, "JPEG".to_string()),
        "png" => (FormatType::NativeCjxl, "PNG".to_string()),
        "apng" => (FormatType::NativeCjxl, "APNG".to_string()),
        "gif" => (FormatType::NativeCjxl, "GIF".to_string()),
        "exr" => (FormatType::NativeCjxl, "OpenEXR".to_string()),
        "ppm" | "pnm" | "pfm" | "pam" | "pgx" => (FormatType::NativeCjxl, ext.to_uppercase()),
        "webp" => (FormatType::Extended, "WebP".to_string()),
        "avif" => (FormatType::Extended, "AVIF".to_string()),
        "tiff" | "tif" => (FormatType::Extended, "TIFF".to_string()),
        "bmp" => (FormatType::Extended, "BMP".to_string()),
        "ico" => (FormatType::Extended, "ICO".to_string()),
        "tga" => (FormatType::Extended, "TGA".to_string()),
        "hdr" => (FormatType::Extended, "HDR".to_string()),
        "cr2" | "nef" | "arw" | "dng" | "rw2" | "orf" => {
            (FormatType::Raw, format!("RAW/{}", ext.to_uppercase()))
        }
        _ => (FormatType::Extended, ext.to_uppercase()),
    }
}

pub fn inspect_image(path_str: &str) -> Result<ImageItemInfo, String> {
    let path = Path::new(path_str);
    if !path.is_file() {
        return Err(format!("File does not exist: {}", path_str));
    }

    let orig_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image")
        .to_string();

    let (_fmt_type, format_name) = detect_format_type(path);
    let extracted = extract_metadata(path);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // If it's a JXL file, read dimensions and thumbnail via jxlinfo/djxl
    let (width, height, thumbnail) = if ext == "jxl" {
        read_jxl_info(path)
    } else {
        // Try reading dimensions using image reader with defensive limits
        match image::ImageReader::open(path) {
            Ok(mut reader) => {
                reader.limits(super::limits::create_safe_image_limits());
                match reader.with_guessed_format() {
                    Ok(format_reader) => match format_reader.into_dimensions() {
                        Ok((w, h)) => {
                            let thumb = generate_thumbnail(path);
                            (w, h, thumb)
                        }
                        Err(_) => {
                            if let Ok(dyn_img) = super::limits::open_image_safely(path) {
                                let (w, h) = (dyn_img.width(), dyn_img.height());
                                let thumb = create_thumb_from_dyn(&dyn_img);
                                (w, h, Some(thumb))
                            } else {
                                (3840, 2160, None)
                            }
                        }
                    },
                    Err(_) => (3840, 2160, None),
                }
            }
            Err(_) => (3840, 2160, None),
        }
    };

    // Enforce defensive gatekeeper dimension ceiling and memory limits
    if let Err(e) = super::limits::validate_image_dimensions(width, height, 4) {
        return Err(e.to_string());
    }

    let id = format!(
        "item_{:x}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) ^ (orig_size as i64)
    );

    Ok(ImageItemInfo {
        id,
        path: path_str.to_string(),
        name,
        orig_size,
        format: format_name,
        width,
        height,
        thumbnail,
        metadata: extracted.summary,
    })
}

pub fn inspect_image_without_thumbnail(path_str: &str) -> Result<ImageItemInfo, String> {
    let path = Path::new(path_str);
    if !path.is_file() {
        return Err(format!("File does not exist: {}", path_str));
    }

    let orig_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image")
        .to_string();

    let (_fmt_type, format_name) = detect_format_type(path);
    let extracted = extract_metadata(path);

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let (width, height) = if ext == "jxl" {
        read_jxl_dimensions(path)
    } else {
        match image::ImageReader::open(path) {
            Ok(mut reader) => {
                reader.limits(super::limits::create_safe_image_limits());
                match reader.with_guessed_format() {
                    Ok(format_reader) => match format_reader.into_dimensions() {
                        Ok((w, h)) => (w, h),
                        Err(_) => (3840, 2160),
                    },
                    Err(_) => (3840, 2160),
                }
            }
            Err(_) => (3840, 2160),
        }
    };

    if let Err(e) = super::limits::validate_image_dimensions(width, height, 4) {
        return Err(e.to_string());
    }

    let id = format!(
        "item_{:x}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) ^ (orig_size as i64)
    );

    Ok(ImageItemInfo {
        id,
        path: path_str.to_string(),
        name,
        orig_size,
        format: format_name,
        width,
        height,
        thumbnail: None,
        metadata: extracted.summary,
    })
}


fn generate_thumbnail(path: &Path) -> Option<String> {
    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        if let Ok(mut img_reader) = image::ImageReader::new(reader).with_guessed_format() {
            img_reader.limits(super::limits::create_safe_image_limits());
            if let Ok(img) = img_reader.decode() {
                return Some(create_thumb_from_dyn(&img));
            }
        }
    }
    None
}

fn create_thumb_from_dyn(img: &image::DynamicImage) -> String {
    let thumb = img.resize(96, 96, FilterType::Triangle);
    let mut buffer = Cursor::new(Vec::new());
    if thumb
        .write_to(&mut buffer, image::ImageFormat::Jpeg)
        .is_ok()
    {
        let base64_str = BASE64.encode(buffer.into_inner());
        format!("data:image/jpeg;base64,{}", base64_str)
    } else {
        "".to_string()
    }
}

pub fn prepare_extended_format(
    source_path: &Path,
    temp_dir: &Path,
) -> Result<PathBuf, String> {
    let dyn_img = super::limits::open_image_safely(source_path)
        .map_err(|e| format!("Failed to decode extended image: {}", e))?;

    let raw_stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("temp");
    let stem = super::jxl::sanitize_filename(raw_stem);

    let temp_img_path = temp_dir.join(format!("{}_converted.png", stem));
    dyn_img
        .save_with_format(&temp_img_path, image::ImageFormat::Png)
        .map_err(|e| format!("Failed to write intermediate PNG: {}", e))?;

    Ok(temp_img_path)
}

pub fn scan_folder_for_images(dir_path: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![dir_path.to_path_buf()];

    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    match ext.as_str() {
                        "png" | "jpg" | "jpeg" | "webp" | "avif" | "tiff" | "tif" | "bmp"
                        | "gif" | "exr" | "hdr" | "ico" | "tga" | "cr2" | "nef" | "arw" | "dng"
                        | "rw2" | "orf" | "ppm" | "pnm" | "pfm" | "pam" | "pgx" => {
                            results.push(path);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    results.sort();
    results
}

pub fn scan_folder_for_jxl(dir_path: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![dir_path.to_path_buf()];

    while let Some(current) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&current) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.is_file() {
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if ext == "jxl" {
                        results.push(path);
                    }
                }
            }
        }
    }
    results.sort();
    results
}

pub fn read_jxl_dimensions(path: &Path) -> (u32, u32) {
    let mut width = 1920;
    let mut height = 1080;

    if let Some(jxlinfo) = super::detector::find_jxlinfo_executable() {
        let mut cmd = std::process::Command::new(jxlinfo);
        cmd.arg(path);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        if let Ok(output) = cmd.output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("Intrinsic dimensions:") || line.contains("JPEG XL image,") {
                    for part in line.split(|c: char| !c.is_alphanumeric()) {
                        if let Some((w_s, h_s)) = part.split_once('x')
                            && let (Ok(w), Ok(h)) = (w_s.parse::<u32>(), h_s.parse::<u32>())
                            && w > 0
                            && h > 0
                        {
                            width = w;
                            height = h;
                            break;
                        }
                    }
                }
            }
        }
    }

    (width, height)
}

fn read_jxl_info(path: &Path) -> (u32, u32, Option<String>) {
    let (width, height) = read_jxl_dimensions(path);
    let mut thumb = None;

    if let Some(djxl) = super::detector::find_djxl_executable()
        && let Ok(temp_dir) = tempfile::tempdir()
    {
        let thumb_path = temp_dir.path().join("thumb.png");
        let mut cmd = std::process::Command::new(djxl);
        cmd.arg(path).arg(&thumb_path).arg("-s").arg("8").arg("--quiet");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        if let Ok(status) = cmd.status()
            && status.success()
            && thumb_path.is_file()
            && let Ok(dyn_img) = image::open(&thumb_path)
        {
            thumb = Some(create_thumb_from_dyn(&dyn_img));
        }
    }

    (width, height, thumb)
}
