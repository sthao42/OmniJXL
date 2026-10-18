use omnijxl_lib::engine::jxl::sanitize_filename;
use omnijxl_lib::engine::metadata::extract_metadata;
use std::fs::File;
use std::io::Write;

#[test]
fn test_sanitize_filename_prevents_path_traversal() {
    assert_eq!(sanitize_filename("../../evil"), "evil");
    assert_eq!(sanitize_filename("..\\..\\secret"), "secret");
    assert_eq!(sanitize_filename(".."), "output");
    assert_eq!(sanitize_filename("."), "output");
    assert_eq!(sanitize_filename(""), "output");
    assert_eq!(sanitize_filename("   "), "output");
    assert_eq!(sanitize_filename("normal_photo"), "normal_photo");
    assert_eq!(
        sanitize_filename("photo:name*with?bad<chars>|"),
        "photo_name_with_bad_chars"
    );
}

#[test]
fn test_metadata_extraction_survives_malformed_jpeg() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    // Case 1: 0-byte file
    let empty_path = temp_dir.path().join("empty.jpg");
    File::create(&empty_path).unwrap();
    let res = extract_metadata(&empty_path);
    assert!(!res.summary.has_exif);

    // Case 2: Incomplete 2-byte JPEG header
    let header_only = temp_dir.path().join("header.jpg");
    let mut f = File::create(&header_only).unwrap();
    f.write_all(&[0xFF, 0xD8]).unwrap();
    let res = extract_metadata(&header_only);
    assert!(!res.summary.has_exif);

    // Case 3: Malformed APP1 marker with length < 2 (regression test for slice panic)
    let corrupt_marker = temp_dir.path().join("corrupt_marker.jpg");
    let mut f = File::create(&corrupt_marker).unwrap();
    // 0xFF 0xD8 (SOI), 0xFF 0xE1 (APP1), 0x00 0x01 (Length = 1: invalid! Length must be >= 2)
    f.write_all(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x01, 0xFF, 0xD9])
        .unwrap();
    let res = extract_metadata(&corrupt_marker);
    assert!(!res.summary.has_exif); // Successfully processed without panic!

    // Case 4: Length exceeds EOF
    let overflow_marker = temp_dir.path().join("overflow.jpg");
    let mut f = File::create(&overflow_marker).unwrap();
    f.write_all(&[0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xFF, 0x00, 0x00])
        .unwrap();
    let res = extract_metadata(&overflow_marker);
    assert!(!res.summary.has_exif);
}

#[test]
fn test_metadata_extraction_survives_malformed_png() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    // PNG with chunk claiming 4GB length
    let corrupt_png = temp_dir.path().join("corrupt.png");
    let mut f = File::create(&corrupt_png).unwrap();
    let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    data.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]); // 4GB chunk
    data.extend_from_slice(b"eXIf");
    f.write_all(&data).unwrap();

    let res = extract_metadata(&corrupt_png);
    assert!(!res.summary.has_exif); // Handled cleanly without overflow or panic
}

#[test]
fn test_metadata_extraction_survives_malformed_webp() {
    let temp_dir = tempfile::tempdir().expect("tempdir");

    let corrupt_webp = temp_dir.path().join("corrupt.webp");
    let mut f = File::create(&corrupt_webp).unwrap();
    // RIFF header with invalid length
    f.write_all(b"RIFF\x20\x00\x00\x00WEBPEXIF\xFF\xFF\xFF\xFF")
        .unwrap();

    let res = extract_metadata(&corrupt_webp);
    assert!(!res.summary.has_exif);
}

#[test]
fn test_flag_injection_prevention_on_filenames() {
    let input_path = std::path::Path::new("-v.png");
    let abs = std::path::absolute(input_path).expect("absolute path");
    let abs_str = abs.to_string_lossy();
    assert!(
        !abs_str.starts_with("-"),
        "Absolute path must not start with a hyphen: {}",
        abs_str
    );
}

#[test]
fn test_security_gatekeeper_dimension_limits() {
    use omnijxl_lib::engine::limits::{ImageSecurityError, validate_image_dimensions};

    // Exactly at ceiling
    assert!(validate_image_dimensions(16_384, 4_000, 4).is_ok());

    // Exceeds width ceiling (> 16,384)
    let res = validate_image_dimensions(16_385, 100, 4);
    assert!(
        matches!(res, Err(ImageSecurityError::ExceedsDimensionLimit { .. })),
        "Width > 16384 must trigger ExceedsDimensionLimit"
    );

    // Exceeds height ceiling (> 16,384)
    let res = validate_image_dimensions(100, 16_385, 4);
    assert!(
        matches!(res, Err(ImageSecurityError::ExceedsDimensionLimit { .. })),
        "Height > 16384 must trigger ExceedsDimensionLimit"
    );

    // Extreme 50,000x50,000 decompression bomb
    let res = validate_image_dimensions(50_000, 50_000, 4);
    assert!(
        matches!(res, Err(ImageSecurityError::ExceedsDimensionLimit { .. })),
        "50,000x50,000 must be blocked immediately before allocation"
    );
}

#[test]
fn test_security_gatekeeper_decompression_bomb_memory_limit() {
    use omnijxl_lib::engine::limits::{ImageSecurityError, validate_image_dimensions};

    // 10,000 x 10,000 x 4 bytes = 400,000,000 bytes (~381 MB > 256 MB ceiling)
    let res = validate_image_dimensions(10_000, 10_000, 4);
    assert!(
        matches!(res, Err(ImageSecurityError::ExceedsMemoryLimit { .. })),
        "Buffer requiring >256MB must trigger ExceedsMemoryLimit"
    );

    // Standard 4K image: 3840 x 2160 x 4 bytes = ~33 MB <= 256 MB (allowed)
    assert!(validate_image_dimensions(3840, 2160, 4).is_ok());

    // 8K image: 7680 x 4320 x 4 bytes = ~132 MB <= 256 MB (allowed)
    assert!(validate_image_dimensions(7680, 4320, 4).is_ok());
}

#[test]
fn test_security_sniff_magic_bytes_detection() {
    use omnijxl_lib::engine::limits::{MagicFormat, sniff_magic_bytes};

    // JPEG
    assert_eq!(sniff_magic_bytes(&[0xFF, 0xD8, 0xFF, 0xE0]), MagicFormat::Jpeg);

    // PNG
    assert_eq!(
        sniff_magic_bytes(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]),
        MagicFormat::Png
    );

    // JXL Bare Codestream
    assert_eq!(sniff_magic_bytes(&[0xFF, 0x0A, 0x00, 0x00]), MagicFormat::JxlCodestream);

    // JXL ISOBMFF Container
    assert_eq!(
        sniff_magic_bytes(&[
            0x00, 0x00, 0x00, 0x0C, b'J', b'X', b'L', b' ', 0x0D, 0x0A, 0x87, 0x0A
        ]),
        MagicFormat::JxlContainer
    );

    // WebP
    assert_eq!(
        sniff_magic_bytes(b"RIFF\x10\x00\x00\x00WEBPVP8 "),
        MagicFormat::WebP
    );

    // BMP
    assert_eq!(sniff_magic_bytes(b"BM\x36\x00\x00\x00"), MagicFormat::Bmp);

    // GIF
    assert_eq!(sniff_magic_bytes(b"GIF89a\x01\x00"), MagicFormat::Gif);

    // Random bytes
    assert_eq!(sniff_magic_bytes(&[0x12, 0x34, 0x56, 0x78]), MagicFormat::Unknown);
}
