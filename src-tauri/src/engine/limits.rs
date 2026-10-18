use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub const MAX_IMAGE_WIDTH: u32 = 16_384;
pub const MAX_IMAGE_HEIGHT: u32 = 16_384;
pub const MAX_DECOMPRESSED_BUFFER_BYTES: u64 = 256 * 1024 * 1024; // 256 MB

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageSecurityError {
    ExceedsDimensionLimit { width: u32, height: u32, max: u32 },
    ExceedsMemoryLimit { required_bytes: u64, max_bytes: u64 },
    InvalidMagicBytes { detected: String },
    CorruptedHeader(String),
    IoError(String),
}

impl std::fmt::Display for ImageSecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExceedsDimensionLimit { width, height, max } => {
                write!(f, "Image dimensions ({}x{}) exceed maximum safety limit of {}x{}", width, height, max, max)
            }
            Self::ExceedsMemoryLimit { required_bytes, max_bytes } => {
                write!(
                    f,
                    "Decompressed buffer allocation ({} MB) exceeds maximum security ceiling of {} MB (potential decompression bomb)",
                    required_bytes / (1024 * 1024),
                    max_bytes / (1024 * 1024)
                )
            }
            Self::InvalidMagicBytes { detected } => {
                write!(f, "File content header does not match expected image magic bytes (detected: {})", detected)
            }
            Self::CorruptedHeader(msg) => write!(f, "Corrupted image header: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for ImageSecurityError {}

/// Calculates pre-allocation memory requirement and validates against security limits
pub fn validate_image_dimensions(
    width: u32,
    height: u32,
    bytes_per_pixel: u32,
) -> Result<(), ImageSecurityError> {
    if width > MAX_IMAGE_WIDTH || height > MAX_IMAGE_HEIGHT {
        return Err(ImageSecurityError::ExceedsDimensionLimit {
            width,
            height,
            max: MAX_IMAGE_WIDTH,
        });
    }

    let bpp = if bytes_per_pixel == 0 { 4 } else { bytes_per_pixel } as u64;
    let required_bytes = (width as u64)
        .checked_mul(height as u64)
        .and_then(|pixels| pixels.checked_mul(bpp))
        .ok_or(ImageSecurityError::ExceedsMemoryLimit {
            required_bytes: u64::MAX,
            max_bytes: MAX_DECOMPRESSED_BUFFER_BYTES,
        })?;

    if required_bytes > MAX_DECOMPRESSED_BUFFER_BYTES {
        return Err(ImageSecurityError::ExceedsMemoryLimit {
            required_bytes,
            max_bytes: MAX_DECOMPRESSED_BUFFER_BYTES,
        });
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicFormat {
    Jpeg,
    Png,
    JxlContainer,
    JxlCodestream,
    WebP,
    Tiff,
    Bmp,
    Gif,
    OpenExr,
    Unknown,
}

/// Inspects the first 16+ bytes of an image to identify real format regardless of file extension
pub fn sniff_magic_bytes(header: &[u8]) -> MagicFormat {
    if header.len() < 2 {
        return MagicFormat::Unknown;
    }

    // JPEG: FF D8 FF
    if header.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return MagicFormat::Jpeg;
    }

    // PNG: 89 50 4E 47 0D 0A 1A 0A
    if header.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return MagicFormat::Png;
    }

    // JXL Bare Codestream: FF 0A
    if header.starts_with(&[0xFF, 0x0A]) {
        return MagicFormat::JxlCodestream;
    }

    // JXL Container Box: 00 00 00 0C 4A 58 4C 20 0D 0A 87 0A
    if header.len() >= 12
        && header[0..4] == [0x00, 0x00, 0x00, 0x0C]
        && &header[4..8] == b"JXL "
        && header[8..12] == [0x0D, 0x0A, 0x87, 0x0A]
    {
        return MagicFormat::JxlContainer;
    }

    // WebP: RIFF .... WEBP
    if header.len() >= 12 && header.starts_with(b"RIFF") && &header[8..12] == b"WEBP" {
        return MagicFormat::WebP;
    }

    // TIFF: II*. or MM.*
    if header.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || header.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return MagicFormat::Tiff;
    }

    // BMP: BM
    if header.starts_with(b"BM") {
        return MagicFormat::Bmp;
    }

    // GIF: GIF87a or GIF89a
    if header.starts_with(b"GIF87a") || header.starts_with(b"GIF89a") {
        return MagicFormat::Gif;
    }

    // OpenEXR: 76 2F 31 01
    if header.starts_with(&[0x76, 0x2F, 0x31, 0x01]) {
        return MagicFormat::OpenExr;
    }

    MagicFormat::Unknown
}

/// Sniffs magic bytes from file on disk
pub fn sniff_file_magic(path: &Path) -> Result<MagicFormat, ImageSecurityError> {
    let mut file = File::open(path).map_err(|e| ImageSecurityError::IoError(e.to_string()))?;
    let mut buf = [0u8; 16];
    let bytes_read = file.read(&mut buf).map_err(|e| ImageSecurityError::IoError(e.to_string()))?;
    Ok(sniff_magic_bytes(&buf[..bytes_read]))
}

/// Builds an image::Limits configuration matching the gatekeeper ceilings
pub fn create_safe_image_limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_IMAGE_WIDTH);
    limits.max_image_height = Some(MAX_IMAGE_HEIGHT);
    limits.max_alloc = Some(MAX_DECOMPRESSED_BUFFER_BYTES);
    limits
}

/// Opens an image using strict safety limits to prevent decompression bombs
pub fn open_image_safely(path: &Path) -> Result<image::DynamicImage, ImageSecurityError> {
    let file = File::open(path).map_err(|e| ImageSecurityError::IoError(e.to_string()))?;
    let reader = BufReader::new(file);

    let mut img_reader = image::ImageReader::new(reader)
        .with_guessed_format()
        .map_err(|e| ImageSecurityError::CorruptedHeader(e.to_string()))?;

    img_reader.limits(create_safe_image_limits());

    // Pre-check dimensions before allocating full dynamic image
    if let Ok((width, height)) = img_reader.into_dimensions() {
        validate_image_dimensions(width, height, 4)?;
    }

    // Re-open with limits for decode
    let file2 = File::open(path).map_err(|e| ImageSecurityError::IoError(e.to_string()))?;
    let mut img_reader2 = image::ImageReader::new(BufReader::new(file2))
        .with_guessed_format()
        .map_err(|e| ImageSecurityError::CorruptedHeader(e.to_string()))?;
    img_reader2.limits(create_safe_image_limits());

    img_reader2.decode().map_err(|e| match e {
        image::ImageError::Limits(_) => {
            ImageSecurityError::ExceedsMemoryLimit {
                required_bytes: MAX_DECOMPRESSED_BUFFER_BYTES + 1,
                max_bytes: MAX_DECOMPRESSED_BUFFER_BYTES,
            }
        }
        other => ImageSecurityError::CorruptedHeader(other.to_string()),
    })
}
