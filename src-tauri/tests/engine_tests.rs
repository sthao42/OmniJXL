use omnijxl_lib::engine::detector::{detect_simd, get_system_info};
use omnijxl_lib::engine::formats::{FormatType, detect_format_type, inspect_image};
use omnijxl_lib::engine::jxl::{EncodingProfile, convert_single_image, resolve_destination_folder};
use std::path::Path;

#[test]
fn test_system_and_simd_detection() {
    let simd = detect_simd();
    assert!(
        !simd.is_empty(),
        "SIMD detection should return a non-empty string"
    );
    println!("Detected SIMD: {}", simd);

    let info = get_system_info();
    assert!(
        info.threads > 0,
        "System info should report at least 1 thread"
    );
    println!("System info: {:?}", info);
}

#[test]
fn test_format_detection() {
    assert_eq!(
        detect_format_type(Path::new("test.png")).0,
        FormatType::NativeCjxl
    );
    assert_eq!(
        detect_format_type(Path::new("photo.jpg")).0,
        FormatType::NativeCjxl
    );
    assert_eq!(
        detect_format_type(Path::new("graphic.webp")).0,
        FormatType::Extended
    );
    assert_eq!(
        detect_format_type(Path::new("scan.tiff")).0,
        FormatType::Extended
    );
    assert_eq!(
        detect_format_type(Path::new("camera.cr2")).0,
        FormatType::Raw
    );
}

#[test]
fn test_destination_folder_resolution() {
    let custom = Some("C:\\OmniExports".to_string());
    let resolved = resolve_destination_folder(&custom, Path::new("C:\\source\\img.png"));
    assert_eq!(resolved, Path::new("C:\\OmniExports"));

    let empty = None;
    let fallback = resolve_destination_folder(&empty, Path::new("C:\\source\\img.png"));
    assert!(!fallback.to_string_lossy().is_empty());
}

#[test]
fn test_live_image_conversion_to_jxl() {
    let sys_info = get_system_info();
    if !sys_info.is_ready {
        println!("Skipping live conversion test because cjxl is not ready");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_img_path = temp_dir.path().join("test_sample.png");

    // Generate a valid 256x256 test RGB PNG image
    let mut imgbuf = image::RgbImage::new(256, 256);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        let r = (x % 256) as u8;
        let g = (y % 256) as u8;
        let b = ((x + y) % 256) as u8;
        *pixel = image::Rgb([r, g, b]);
    }
    imgbuf
        .save(&test_img_path)
        .expect("Failed to save test PNG");

    // Inspect image
    let item_info =
        inspect_image(&test_img_path.to_string_lossy()).expect("Failed to inspect test image");
    assert_eq!(item_info.width, 256);
    assert_eq!(item_info.height, 256);

    // Convert Lossless
    let mut profile = EncodingProfile {
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        is_lossless: true,
        effort: 5,
        ..Default::default()
    };

    let res = convert_single_image(&item_info, &profile);
    assert_eq!(
        res.status, "done",
        "Conversion should succeed: {:?}",
        res.error_message
    );
    assert!(res.target_path.is_some());
    assert!(res.target_size.unwrap_or(0) > 0);
    println!(
        "Lossless conversion succeeded! Target size: {:?} bytes, time: {:?} ms",
        res.target_size, res.time_ms
    );

    // Convert Lossy (Butteraugli distance 1.0)
    profile.is_lossless = false;
    profile.distance = 1.0;
    let res_lossy = convert_single_image(&item_info, &profile);
    assert_eq!(
        res_lossy.status, "done",
        "Lossy conversion should succeed: {:?}",
        res_lossy.error_message
    );
    println!(
        "Lossy conversion succeeded! Target size: {:?} bytes, time: {:?} ms",
        res_lossy.target_size, res_lossy.time_ms
    );
}

#[test]
fn test_live_jpeg_reversible_transcode() {
    let sys_info = get_system_info();
    if !sys_info.is_ready {
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_jpg_path = temp_dir.path().join("camera_photo.jpg");

    // Create synthetic JPEG
    let mut imgbuf = image::RgbImage::new(192, 192);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x * 2) as u8, (y * 2) as u8, 128]);
    }
    imgbuf
        .save(&test_jpg_path)
        .expect("Failed to save test JPEG");

    let item_info = inspect_image(&test_jpg_path.to_string_lossy()).expect("inspect JPEG");
    assert_eq!(item_info.width, 192);
    assert_eq!(item_info.height, 192);

    let profile = EncodingProfile {
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        reversible_jpeg: true,
        is_lossless: true,
        ..Default::default()
    };

    let res = convert_single_image(&item_info, &profile);
    assert_eq!(
        res.status, "done",
        "JPEG reversible transcode failed: {:?}",
        res.error_message
    );
    assert!(res.target_size.unwrap_or(0) > 0);
    println!(
        "Reversible JPEG transcode target size: {:?} bytes",
        res.target_size
    );
}

#[test]
fn test_live_extended_bmp_conversion() {
    let sys_info = get_system_info();
    if !sys_info.is_ready {
        return;
    }

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let test_bmp_path = temp_dir.path().join("legacy_graphic.bmp");

    let mut imgbuf = image::RgbImage::new(128, 128);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgb([x as u8, y as u8, 200]);
    }
    imgbuf
        .save(&test_bmp_path)
        .expect("Failed to save test BMP");

    let item_info = inspect_image(&test_bmp_path.to_string_lossy()).expect("inspect BMP");

    let profile = EncodingProfile {
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        is_lossless: false,
        distance: 2.0,
        effort: 4,
        is_progressive: true,
        ..Default::default()
    };

    let res = convert_single_image(&item_info, &profile);
    assert_eq!(
        res.status, "done",
        "Extended BMP conversion failed: {:?}",
        res.error_message
    );
    assert!(res.target_size.unwrap_or(0) > 0);
    println!(
        "BMP extended conversion target size: {:?} bytes",
        res.target_size
    );
}

#[test]
fn test_encoding_profile_json_roundtrip() {
    let original = EncodingProfile {
        is_lossless: false,
        distance: 1.5,
        effort: 8,
        is_progressive: true,
        preserve_metadata: true,
        reversible_jpeg: false,
        dest_folder: Some("C:\\Users\\Test\\Exports".to_string()),
    };

    let serialized = serde_json::to_string(&original).expect("serialize profile");
    assert!(serialized.contains("\"distance\":1.5"));
    assert!(serialized.contains("\"effort\":8"));

    let deserialized: EncodingProfile =
        serde_json::from_str(&serialized).expect("deserialize profile");
    assert_eq!(deserialized.is_lossless, original.is_lossless);
    assert_eq!(deserialized.distance, original.distance);
    assert_eq!(deserialized.effort, original.effort);
    assert_eq!(deserialized.dest_folder, original.dest_folder);
}

#[test]
fn test_system_info_json_roundtrip() {
    let sys_info = get_system_info();
    let json_str = serde_json::to_string(&sys_info).expect("serialize system info");
    assert!(json_str.contains("\"threads\":"));
    assert!(json_str.contains("\"simd\":"));

    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("parse JSON value");
    assert!(parsed.get("threads").unwrap().as_u64().unwrap() > 0);
    assert!(parsed.get("simd").unwrap().is_string());
}

#[test]
fn test_inspect_image_produces_valid_thumbnail() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_path = temp_dir.path().join("thumb_test.png");

    let mut imgbuf = image::RgbImage::new(64, 64);
    for pixel in imgbuf.pixels_mut() {
        *pixel = image::Rgb([120, 150, 220]);
    }
    imgbuf.save(&test_path).expect("save thumb test PNG");

    let item_info = inspect_image(&test_path.to_string_lossy()).expect("inspect image");
    assert_eq!(item_info.width, 64);
    assert_eq!(item_info.height, 64);
    let thumb = item_info
        .thumbnail
        .expect("Thumbnail should be generated for valid image");
    assert!(
        thumb.starts_with("data:image/jpeg;base64,"),
        "Thumbnail must be a base64 JPEG data URI"
    );
    assert!(thumb.len() > 50);
}

#[test]
fn test_scan_folder_for_images_detects_subdirectories_and_filters() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let root = temp_dir.path();
    let sub = root.join("subfolder");
    std::fs::create_dir(&sub).unwrap();

    // Create image files
    std::fs::write(root.join("img1.png"), b"fake png").unwrap();
    std::fs::write(root.join("img2.JPG"), b"fake jpg").unwrap();
    std::fs::write(sub.join("img3.webp"), b"fake webp").unwrap();

    // Create non-image files (should be ignored)
    std::fs::write(root.join("notes.txt"), b"text file").unwrap();
    std::fs::write(sub.join("data.json"), b"{}").unwrap();

    let found = omnijxl_lib::engine::formats::scan_folder_for_images(root);
    assert_eq!(found.len(), 3, "Should discover exactly 3 images");

    let filenames: Vec<String> = found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
        .collect();
    assert!(filenames.contains(&"img1.png".to_string()));
    assert!(filenames.contains(&"img2.JPG".to_string()));
    assert!(filenames.contains(&"img3.webp".to_string()));
}

fn calc_crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFFFFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB88320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

fn append_png_text_chunk(png: &[u8], keyword: &str, text: &str) -> Vec<u8> {
    let mut chunk_payload = Vec::new();
    chunk_payload.extend_from_slice(keyword.as_bytes());
    chunk_payload.push(0);
    chunk_payload.extend_from_slice(text.as_bytes());

    let len = chunk_payload.len() as u32;
    let mut to_crc = Vec::new();
    to_crc.extend_from_slice(b"tEXt");
    to_crc.extend_from_slice(&chunk_payload);
    let crc = calc_crc32(&to_crc);

    let insert_pos = png.len().saturating_sub(12);
    let mut out = Vec::new();
    out.extend_from_slice(&png[..insert_pos]);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(b"tEXt");
    out.extend_from_slice(&chunk_payload);
    out.extend_from_slice(&crc.to_be_bytes());
    out.extend_from_slice(&png[insert_pos..]);
    out
}

#[test]
fn test_comfyui_png_workflow_and_metadata_preservation() {
    let sys_info = get_system_info();
    if !sys_info.is_ready {
        println!("Skipping ComfyUI workflow test because cjxl is not available");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let png_path = temp_dir.path().join("comfyui_test.png");

    // 1. Create RGBA image with alpha channel
    let mut img = image::RgbaImage::new(64, 64);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let alpha = if (x + y) % 2 == 0 { 255 } else { 128 };
        *pixel = image::Rgba([200, 50, 100, alpha]);
    }
    let mut raw_png = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut raw_png);
    img.write_to(&mut cursor, image::ImageFormat::Png).expect("write PNG");

    // 2. Add ComfyUI prompt and workflow text chunks
    let prompt_json = r#"{"3": {"class_type": "KSampler", "inputs": {"seed": 42}}}"#;
    let workflow_json = r#"{"nodes": [{"id": 3, "type": "KSampler"}], "links": []}"#;
    let png_with_prompt = append_png_text_chunk(&raw_png, "prompt", prompt_json);
    let png_with_both = append_png_text_chunk(&png_with_prompt, "workflow", workflow_json);
    std::fs::write(&png_path, &png_with_both).expect("write comfy png");

    // 3. Inspect image and verify ComfyUI detection
    let inspected = inspect_image(&png_path.to_string_lossy()).expect("inspect comfy PNG");
    assert!(inspected.metadata.has_comfy_workflow, "Metadata must detect ComfyUI workflow");
    assert_eq!(inspected.metadata.comfy_nodes_count, Some(1), "Should count 1 node");

    // 4. Convert to JXL with preserve_metadata: true
    let profile = EncodingProfile {
        is_lossless: true,
        preserve_metadata: true,
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        ..Default::default()
    };

    let result = convert_single_image(&inspected, &profile);
    assert_eq!(result.status, "done", "Conversion must succeed: {:?}", result.error_message);

    let jxl_path = std::path::PathBuf::from(result.target_path.expect("target_path"));
    assert!(jxl_path.is_file(), "JXL file must exist");

    // 5. Verify JXL container and ComfyUI brob box contents
    let jxl_bytes = std::fs::read(&jxl_path).expect("read JXL");
    assert!(jxl_bytes.len() >= 32);
    assert_eq!(&jxl_bytes[4..8], b"JXL ");

    // Verify brob box is present right after ftyp (offset 32)
    assert_eq!(&jxl_bytes[36..40], b"brob", "brob box must be present right after ftyp");
    assert_eq!(&jxl_bytes[40..44], b"comf", "brob payload FourCC must be 'comf'");

    let brob_box_len = u32::from_be_bytes([jxl_bytes[32], jxl_bytes[33], jxl_bytes[34], jxl_bytes[35]]) as usize;
    let compressed_payload = &jxl_bytes[44..32 + brob_box_len];

    // Decompress brotli payload
    let mut decompressed = Vec::new();
    let mut reader = brotli::Decompressor::new(compressed_payload, 4096);
    std::io::Read::read_to_end(&mut reader, &mut decompressed).expect("decompress brotli payload");

    let parsed_meta: serde_json::Value = serde_json::from_slice(&decompressed).expect("parse JSON");
    assert!(parsed_meta.get("prompt").is_some(), "Preserved JSON must have prompt");
    assert!(parsed_meta.get("workflow").is_some(), "Preserved JSON must have workflow");

    println!("Successfully verified ComfyUI workflow preserved inside JXL container!");
}

#[test]
fn test_depth_map_16bit_preservation() {
    let sys_info = get_system_info();
    if !sys_info.is_ready {
        println!("Skipping depth map test because cjxl is not available");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let depth_path = temp_dir.path().join("depth_map_16bit.png");

    // Create 16-bit grayscale depth image (Luma16)
    let mut img = image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::new(64, 64);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let val = ((x * 1024 + y * 1024) % 65536) as u16;
        *pixel = image::Luma([val]);
    }
    img.save(&depth_path).expect("save 16-bit depth PNG");

    let inspected = inspect_image(&depth_path.to_string_lossy()).expect("inspect depth image");
    assert_eq!(inspected.width, 64);
    assert_eq!(inspected.height, 64);

    let profile = EncodingProfile {
        is_lossless: true,
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        ..Default::default()
    };

    let result = convert_single_image(&inspected, &profile);
    assert_eq!(result.status, "done", "16-bit conversion must succeed: {:?}", result.error_message);

    let jxl_path = std::path::PathBuf::from(result.target_path.expect("target_path"));
    assert!(jxl_path.is_file(), "JXL file must exist");
    println!("Successfully verified 16-bit depth map conversion!");
}

#[test]
fn test_convert_jxl_to_legacy_png_restores_workflow() {
    use omnijxl_lib::engine::legacy::{LegacyExportProfile, convert_single_jxl_to_legacy};
    use omnijxl_lib::engine::metadata::append_png_text_chunk;

    let sys_info = get_system_info();
    if !sys_info.is_ready || sys_info.djxl_path.is_none() {
        println!("Skipping legacy conversion test because cjxl or djxl is not ready");
        return;
    }

    let temp_dir = tempfile::tempdir().expect("tempdir");
    let test_img_path = temp_dir.path().join("comfy_for_legacy.png");

    // 1. Create a 64x64 RGBA test PNG
    let mut imgbuf = image::RgbaImage::new(64, 64);
    for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
        *pixel = image::Rgba([(x * 4) as u8, (y * 4) as u8, 128, 255]);
    }
    imgbuf.save(&test_img_path).expect("save PNG");

    // 2. Inject ComfyUI metadata into the source PNG
    let sample_prompt = r#"{"3": {"class_type": "KSampler", "inputs": {"seed": 42}}}"#;
    let sample_workflow = r#"{"nodes": [{"id": 3, "type": "KSampler"}], "version": 0.4}"#;

    let png_bytes = std::fs::read(&test_img_path).expect("read PNG");
    let mut modified_png = append_png_text_chunk(&png_bytes, "prompt", sample_prompt);
    modified_png = append_png_text_chunk(&modified_png, "workflow", sample_workflow);
    std::fs::write(&test_img_path, &modified_png).expect("write modified PNG");

    // 3. Inspect source PNG and verify ComfyUI workflow detected
    let inspected_png = inspect_image(&test_img_path.to_string_lossy()).expect("inspect source PNG");
    assert!(inspected_png.metadata.has_comfy_workflow, "Source PNG must have ComfyUI metadata detected");

    // 4. Convert source PNG to JXL
    let encode_profile = EncodingProfile {
        is_lossless: true,
        dest_folder: Some(temp_dir.path().to_string_lossy().to_string()),
        ..Default::default()
    };

    let encode_res = convert_single_image(&inspected_png, &encode_profile);
    assert_eq!(encode_res.status, "done", "Encode to JXL must succeed");
    let jxl_path = std::path::PathBuf::from(encode_res.target_path.expect("jxl path"));
    assert!(jxl_path.is_file(), "JXL file must exist");

    // Inspect the generated JXL
    let inspected_jxl = inspect_image(&jxl_path.to_string_lossy()).expect("inspect JXL");
    assert_eq!(inspected_jxl.width, 64);
    assert!(inspected_jxl.metadata.has_comfy_workflow, "JXL must retain ComfyUI workflow flag");

    // 5. Convert JXL back to Legacy PNG
    let unpack_dir = temp_dir.path().join("unpacked_legacy");
    let legacy_profile_png = LegacyExportProfile {
        target_format: "png".to_string(),
        jpeg_quality: 95,
        reconstruct_jpeg: true,
        dest_folder: Some(unpack_dir.to_string_lossy().to_string()),
    };

    let legacy_res = convert_single_jxl_to_legacy(&inspected_jxl, &legacy_profile_png);
    assert_eq!(
        legacy_res.status, "done",
        "Legacy conversion to PNG failed: {:?}",
        legacy_res.error_message
    );
    let unpacked_png_path = std::path::PathBuf::from(legacy_res.target_path.expect("legacy target path"));
    assert!(unpacked_png_path.is_file(), "Unpacked PNG file must exist");

    // 6. Inspect unpacked PNG and verify workflow was restored
    let unpacked_info = inspect_image(&unpacked_png_path.to_string_lossy()).expect("inspect unpacked PNG");
    assert_eq!(unpacked_info.width, 64);
    assert_eq!(unpacked_info.height, 64);
    assert!(
        unpacked_info.metadata.has_comfy_workflow,
        "Unpacked PNG must have ComfyUI workflow restored in tEXt chunk"
    );

    // 7. Convert JXL to Legacy JPG
    let legacy_profile_jpg = LegacyExportProfile {
        target_format: "jpg".to_string(),
        jpeg_quality: 90,
        reconstruct_jpeg: true,
        dest_folder: Some(unpack_dir.to_string_lossy().to_string()),
    };
    let legacy_jpg_res = convert_single_jxl_to_legacy(&inspected_jxl, &legacy_profile_jpg);
    assert_eq!(
        legacy_jpg_res.status, "done",
        "Legacy conversion to JPG failed: {:?}",
        legacy_jpg_res.error_message
    );
    let unpacked_jpg_path = std::path::PathBuf::from(legacy_jpg_res.target_path.expect("jpg path"));
    assert!(unpacked_jpg_path.is_file(), "Unpacked JPG file must exist");
    assert!(unpacked_jpg_path.extension().unwrap().to_str().unwrap() == "jpg");

    println!("Successfully verified JXL conversion back to legacy PNG (with ComfyUI workflow) and JPG!");
}

#[test]
fn test_handle_dropped_paths_files_and_folders() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let sub_folder = temp_dir.path().join("sub_images");
    std::fs::create_dir_all(&sub_folder).expect("create sub folder");

    let img1 = sub_folder.join("photo1.png");
    let img2 = sub_folder.join("photo2.jpg");
    let root_img = temp_dir.path().join("root_photo.png");

    let blank = image::RgbImage::new(16, 16);
    blank.save(&img1).expect("save img1");
    blank.save(&img2).expect("save img2");
    blank.save(&root_img).expect("save root_img");

    // Dropping a folder + a file together
    let paths = vec![
        sub_folder.to_string_lossy().to_string(),
        root_img.to_string_lossy().to_string(),
    ];

    let drop_res = omnijxl_lib::commands::cmd_handle_dropped_paths(paths, false);
    assert_eq!(drop_res.total_folders, 1);
    assert_eq!(drop_res.total_files, 1);
    assert!(drop_res.folder_summary.is_some());
    let folder_sum = drop_res.folder_summary.unwrap();
    assert_eq!(folder_sum.image_count, 2);
    // Dropping a folder no longer floods items with individual images! Only root_img is in items:
    assert_eq!(drop_res.items.len(), 1);
    assert_eq!(drop_res.items[0].name, "root_photo.png");
}

#[test]
fn test_scan_folder_summary() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let img1 = temp_dir.path().join("scan1.png");
    let img2 = temp_dir.path().join("scan2.webp");
    let txt = temp_dir.path().join("notes.txt");

    let blank = image::RgbImage::new(8, 8);
    blank.save(&img1).expect("save img1");
    blank.save(&img2).expect("save img2");
    std::fs::write(&txt, "some text").expect("write txt");

    let summary = omnijxl_lib::commands::cmd_scan_folder_summary(temp_dir.path().to_string_lossy().to_string())
        .expect("scan folder summary");
    assert_eq!(summary.image_count, 2);
    assert!(summary.total_bytes > 0);
}

#[test]
fn test_handle_dropped_paths_jxl_mode_filters_non_jxl() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let jxl_file = temp_dir.path().join("sample.jxl");
    let png_file = temp_dir.path().join("sample.png");

    // Write minimal jxl header
    std::fs::write(&jxl_file, b"\xFF\x0A\x00\x00").expect("write jxl");
    let blank = image::RgbImage::new(16, 16);
    blank.save(&png_file).expect("save png");

    let paths = vec![
        jxl_file.to_string_lossy().to_string(),
        png_file.to_string_lossy().to_string(),
    ];

    let drop_res = omnijxl_lib::commands::cmd_handle_dropped_paths(paths, true);
    assert_eq!(drop_res.total_files, 2);
    // Only the .jxl file is inspected and added in JXL mode
    assert_eq!(drop_res.items.len(), 1);
    assert_eq!(drop_res.items[0].format, "JPEG XL");
}


