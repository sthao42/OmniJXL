use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImageMetadataSummary {
    pub make: Option<String>,
    pub model: Option<String>,
    pub date_time: Option<String>,
    pub iso: Option<u32>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub focal_length: Option<String>,
    pub gps_latitude: Option<String>,
    pub gps_longitude: Option<String>,
    pub has_icc: bool,
    pub has_exif: bool,
    pub has_xmp: bool,
    pub has_comfy_workflow: bool,
    pub comfy_nodes_count: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComfyMetadata {
    pub prompt: Option<String>,
    pub workflow: Option<String>,
    pub parameters: Option<String>,
    pub extra_text: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ExtractedMetadata {
    pub summary: ImageMetadataSummary,
    pub raw_exif: Option<Vec<u8>>,
    pub raw_icc: Option<Vec<u8>>,
    pub raw_xmp: Option<Vec<u8>>,
    pub comfy: ComfyMetadata,
}

/// Extracts metadata and raw binary blobs (EXIF, ICC, XMP, ComfyUI text chunks) from an image file
pub fn extract_metadata(path: &Path) -> ExtractedMetadata {
    let mut extracted = ExtractedMetadata::default();

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return extracted,
    };

    let mut buffer = Vec::new();
    if file.read_to_end(&mut buffer).is_err() || buffer.len() < 12 {
        return extracted;
    }

    // Format-specific extraction
    if buffer.starts_with(&[0xFF, 0xD8, 0xFF]) {
        // JPEG format
        extract_from_jpeg(&buffer, &mut extracted);
    } else if buffer.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        // PNG format
        extract_from_png(&buffer, &mut extracted);
    } else if buffer.starts_with(b"RIFF") && buffer.len() > 12 && &buffer[8..12] == b"WEBP" {
        // WebP format
        extract_from_webp(&buffer, &mut extracted);
    } else if buffer.starts_with(b"II*\0") || buffer.starts_with(b"MM\0*") {
        // TIFF format - TIFF is itself a TIFF/EXIF container
        extracted.raw_exif = Some(buffer.clone());
        extracted.summary.has_exif = true;
    } else if buffer.len() >= 12 && &buffer[4..8] == b"JXL " {
        // JXL container format
        extract_from_jxl(&buffer, &mut extracted);
    }

    // Nom-exif parsing for detailed tags (dates, GPS, camera settings)
    if let Ok(ms) = nom_exif::MediaSource::open(path) {
        let mut parser = nom_exif::MediaParser::new();
        if let Ok(iter) = parser.parse_exif(ms) {
            let mut summary = extracted.summary.clone();
            for entry in iter {
                let tag_name = format!("{:?}", entry.tag());
                let value_str = match entry.value() {
                    Some(v) => format!("{}", v),
                    None => continue,
                };
                match tag_name.as_str() {
                    "Make" => {
                        if value_str.starts_with("prompt:") && extracted.comfy.prompt.is_none() {
                            let text = value_str.trim_start_matches("prompt:").trim().to_string();
                            extracted.comfy.prompt = Some(text);
                            summary.has_comfy_workflow = true;
                        } else {
                            summary.make = Some(value_str);
                        }
                    }
                    "Model" => {
                        if value_str.starts_with("workflow:") && extracted.comfy.workflow.is_none() {
                            let text = value_str.trim_start_matches("workflow:").trim().to_string();
                            extracted.comfy.workflow = Some(text);
                            summary.has_comfy_workflow = true;
                        } else {
                            summary.model = Some(value_str);
                        }
                    }
                    "DateTimeOriginal" | "DateTime" => summary.date_time = Some(value_str),
                    "ExposureTime" => summary.exposure_time = Some(value_str),
                    "FNumber" => summary.f_number = Some(value_str),
                    "FocalLength" => summary.focal_length = Some(value_str),
                    "GPSLatitude" => summary.gps_latitude = Some(value_str),
                    "GPSLongitude" => summary.gps_longitude = Some(value_str),
                    _ => {}
                }
            }
            if summary.make.is_some()
                || summary.model.is_some()
                || summary.date_time.is_some()
                || summary.gps_latitude.is_some()
            {
                summary.has_exif = true;
            }
            extracted.summary = summary;
        }
    }

    // Compute nodes count if workflow is present
    if let Some(ref wf) = extracted.comfy.workflow {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(wf)
            && let Some(nodes) = parsed.get("nodes").and_then(|n| n.as_array())
        {
            extracted.summary.comfy_nodes_count = Some(nodes.len());
        }
        extracted.summary.has_comfy_workflow = true;
    }

    extracted.summary.has_exif = extracted.raw_exif.is_some() || extracted.summary.has_exif;
    extracted.summary.has_icc = extracted.raw_icc.is_some();
    extracted.summary.has_xmp = extracted.raw_xmp.is_some();

    extracted
}

fn extract_from_jpeg(data: &[u8], out: &mut ExtractedMetadata) {
    let mut pos = 2; // Skip 0xFF 0xD8
    while pos + 4 <= data.len() {
        if data[pos] != 0xFF {
            break;
        }
        let marker = data[pos + 1];
        if marker == 0xDA || marker == 0xD9 {
            // SOS or EOI
            break;
        }

        let length = u16::from_be_bytes([data[pos + 2], data[pos + 3]]) as usize;
        if length < 2 {
            break;
        }

        let segment_end = match (pos + 2).checked_add(length) {
            Some(end) => end,
            None => break,
        };

        if segment_end > data.len() {
            break;
        }

        let payload_start = pos + 4;
        if payload_start <= segment_end {
            let payload = &data[payload_start..segment_end];

            // APP1: EXIF or XMP
            if marker == 0xE1 {
                if payload.starts_with(b"Exif\0\0") && payload.len() > 6 {
                    out.raw_exif = Some(payload[6..].to_vec());
                    out.summary.has_exif = true;
                } else if payload.starts_with(b"http://ns.adobe.com/xap/1.0/\0") && payload.len() > 29 {
                    out.raw_xmp = Some(payload[29..].to_vec());
                    out.summary.has_xmp = true;
                }
            }
            // APP2: ICC Profile
            else if marker == 0xE2
                && payload.starts_with(b"ICC_PROFILE\0")
                && payload.len() > 14
            {
                out.raw_icc = Some(payload[14..].to_vec());
                out.summary.has_icc = true;
            }
        }

        pos = segment_end;
    }
}

fn extract_from_png(data: &[u8], out: &mut ExtractedMetadata) {
    let mut pos = 8; // Skip PNG 8-byte signature
    while pos + 8 <= data.len() {
        let length =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        let chunk_type = &data[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = match data_start.checked_add(length) {
            Some(end) => end,
            None => break,
        };
        let next_pos = match data_end.checked_add(4) {
            Some(np) => np,
            None => break,
        };

        if data_end > data.len() || next_pos > data.len() {
            break;
        }

        let chunk_data = &data[data_start..data_end];

        if chunk_type == b"eXIf" {
            out.raw_exif = Some(chunk_data.to_vec());
            out.summary.has_exif = true;
        } else if chunk_type == b"iCCP" {
            if let Some(null_idx) = chunk_data.iter().position(|&b| b == 0)
                && null_idx + 2 < chunk_data.len()
            {
                let compressed = &chunk_data[null_idx + 2..];
                let mut decoder = flate2::read::ZlibDecoder::new(compressed);
                let mut decompressed = Vec::new();
                if std::io::Read::read_to_end(&mut decoder, &mut decompressed).is_ok() {
                    out.raw_icc = Some(decompressed);
                    out.summary.has_icc = true;
                }
            }
        } else if chunk_type == b"tEXt" {
            if let Some(null_idx) = chunk_data.iter().position(|&b| b == 0) {
                let keyword = String::from_utf8_lossy(&chunk_data[..null_idx]).to_string();
                let text = String::from_utf8_lossy(&chunk_data[null_idx + 1..]).to_string();
                process_text_chunk(&keyword, text, out);
            }
        } else if chunk_type == b"iTXt" {
            // iTXt: keyword(null), comp_flag(1), comp_method(1), lang(null), trans_key(null), text
            if let Some(null_1) = chunk_data.iter().position(|&b| b == 0) {
                let keyword = String::from_utf8_lossy(&chunk_data[..null_1]).to_string();
                let mut p = null_1 + 1;
                if p + 2 <= chunk_data.len() {
                    let comp_flag = chunk_data[p];
                    p += 2;
                    // Skip lang tag
                    while p < chunk_data.len() && chunk_data[p] != 0 {
                        p += 1;
                    }
                    if p < chunk_data.len() {
                        p += 1;
                    }
                    // Skip translated keyword
                    while p < chunk_data.len() && chunk_data[p] != 0 {
                        p += 1;
                    }
                    if p < chunk_data.len() {
                        p += 1;
                    }
                    if p <= chunk_data.len() {
                        let text_bytes = &chunk_data[p..];
                        let final_bytes = if comp_flag == 1 {
                            let mut decoder = flate2::read::ZlibDecoder::new(text_bytes);
                            let mut decomp = Vec::new();
                            if std::io::Read::read_to_end(&mut decoder, &mut decomp).is_ok() {
                                decomp
                            } else {
                                text_bytes.to_vec()
                            }
                        } else {
                            text_bytes.to_vec()
                        };

                        if keyword == "XML:com.adobe.xmp" {
                            out.raw_xmp = Some(final_bytes);
                            out.summary.has_xmp = true;
                        } else {
                            let text = String::from_utf8_lossy(&final_bytes).to_string();
                            process_text_chunk(&keyword, text, out);
                        }
                    }
                }
            }
        } else if chunk_type == b"zTXt" {
            // zTXt: keyword(null), comp_method(1), compressed text
            if let Some(null_idx) = chunk_data.iter().position(|&b| b == 0) {
                let keyword = String::from_utf8_lossy(&chunk_data[..null_idx]).to_string();
                let p = null_idx + 1;
                if p < chunk_data.len() {
                    let compressed = &chunk_data[p + 1..];
                    let mut decoder = flate2::read::ZlibDecoder::new(compressed);
                    let mut decompressed = Vec::new();
                    if std::io::Read::read_to_end(&mut decoder, &mut decompressed).is_ok() {
                        let text = String::from_utf8_lossy(&decompressed).to_string();
                        process_text_chunk(&keyword, text, out);
                    }
                }
            }
        }

        pos = next_pos;
    }
}

fn process_text_chunk(keyword: &str, text: String, out: &mut ExtractedMetadata) {
    let lower = keyword.to_lowercase();
    if lower == "prompt" {
        out.comfy.prompt = Some(text.clone());
        out.summary.has_comfy_workflow = true;
    } else if lower == "workflow" {
        out.comfy.workflow = Some(text.clone());
        out.summary.has_comfy_workflow = true;
    } else if lower == "parameters" {
        out.comfy.parameters = Some(text.clone());
        out.summary.has_comfy_workflow = true;
    }
    out.comfy.extra_text.insert(keyword.to_string(), text);
}

fn extract_from_webp(data: &[u8], out: &mut ExtractedMetadata) {
    if data.len() < 12 {
        return;
    }
    let mut pos = 12;
    while pos + 8 <= data.len() {
        let fourcc = &data[pos..pos + 4];
        let size = u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
            as usize;
        let chunk_start = pos + 8;
        let chunk_end = match chunk_start.checked_add(size) {
            Some(end) => end,
            None => break,
        };
        if chunk_end > data.len() {
            break;
        }

        let payload = &data[chunk_start..chunk_end];
        if fourcc == b"EXIF" {
            out.raw_exif = Some(payload.to_vec());
            out.summary.has_exif = true;
        } else if fourcc == b"ICCP" {
            out.raw_icc = Some(payload.to_vec());
            out.summary.has_icc = true;
        } else if fourcc == b"XMP " {
            out.raw_xmp = Some(payload.to_vec());
            out.summary.has_xmp = true;
        }

        let pad = size % 2;
        pos = match chunk_end.checked_add(pad) {
            Some(p) => p,
            None => break,
        };
    }
}

/// Builds a standard TIFF/EXIF binary buffer containing ImageDescription, UserComment, Software, and DateTime
pub fn build_comfy_exif(workflow: Option<&str>, prompt: Option<&str>, date_str: Option<&str>) -> Vec<u8> {
    // Little-endian TIFF structure
    let mut buffer = Vec::new();
    // TIFF Header: "II" (little endian), 42 (0x002A), offset to IFD0 = 8
    buffer.extend_from_slice(b"II");
    buffer.extend_from_slice(&42u16.to_le_bytes());
    buffer.extend_from_slice(&8u32.to_le_bytes());

    // Strings to store
    let image_desc = workflow.map(|w| format!("Workflow: {}\0", w)).unwrap_or_else(|| "ComfyUI Workflow\0".to_string());
    let make = "ComfyUI\0".to_string();
    let model = prompt.map(|p| format!("prompt:{}\0", p)).unwrap_or_else(|| "OmniJXL\0".to_string());
    let software = "OmniJXL / ComfyUI\0".to_string();
    let date_val = date_str.map(|d| format!("{}\0", d)).unwrap_or_else(|| "2026:10:18 20:00:00\0".to_string());

    let user_comment_payload = format!(
        "{{\"prompt\": {}, \"workflow\": {}}}",
        prompt.unwrap_or("{}"),
        workflow.unwrap_or("{}")
    );
    let mut user_comment_bytes = b"ASCII\0\0\0".to_vec();
    user_comment_bytes.extend_from_slice(user_comment_payload.as_bytes());

    // IFD0 Entries count: 6 entries
    let ifd0_count: u16 = 6;
    buffer.extend_from_slice(&ifd0_count.to_le_bytes());

    // Placeholder for 6 entries * 12 bytes = 72 bytes
    let entries_start = buffer.len();
    buffer.resize(entries_start + (ifd0_count as usize * 12) + 4, 0); // +4 for next IFD offset (0)

    // Data area starts here
    let mut data_area = Vec::new();

    let write_entry = |tag: u16, typ: u16, count: u32, val_or_offset: u32, idx: usize, buf: &mut [u8]| {
        let offset = entries_start + idx * 12;
        buf[offset..offset + 2].copy_from_slice(&tag.to_le_bytes());
        buf[offset + 2..offset + 4].copy_from_slice(&typ.to_le_bytes());
        buf[offset + 4..offset + 8].copy_from_slice(&count.to_le_bytes());
        buf[offset + 8..offset + 12].copy_from_slice(&val_or_offset.to_le_bytes());
    };

    let base_data_offset = (buffer.len()) as u32;

    // Entry 0: ImageDescription (0x010E, ASCII = 2)
    let desc_offset = base_data_offset + data_area.len() as u32;
    data_area.extend_from_slice(image_desc.as_bytes());

    // Entry 1: Make (0x010F, ASCII = 2)
    let make_offset = base_data_offset + data_area.len() as u32;
    data_area.extend_from_slice(make.as_bytes());

    // Entry 2: Model (0x0110, ASCII = 2)
    let model_offset = base_data_offset + data_area.len() as u32;
    data_area.extend_from_slice(model.as_bytes());

    // Entry 3: Software (0x0131, ASCII = 2)
    let soft_offset = base_data_offset + data_area.len() as u32;
    data_area.extend_from_slice(software.as_bytes());

    // Entry 4: DateTime (0x0132, ASCII = 2)
    let date_offset = base_data_offset + data_area.len() as u32;
    data_area.extend_from_slice(date_val.as_bytes());

    // Exif IFD sub-IFD offset
    let exif_ifd_offset = base_data_offset + data_area.len() as u32;
    // Exif IFD has 1 entry: UserComment (0x9286, UNDEFINED = 7)
    let mut exif_ifd_bytes = Vec::new();
    exif_ifd_bytes.extend_from_slice(&1u16.to_le_bytes()); // 1 entry
    let user_comment_data_offset = exif_ifd_offset + 2 + 12 + 4;
    exif_ifd_bytes.extend_from_slice(&0x9286u16.to_le_bytes()); // tag
    exif_ifd_bytes.extend_from_slice(&7u16.to_le_bytes()); // UNDEFINED
    exif_ifd_bytes.extend_from_slice(&(user_comment_bytes.len() as u32).to_le_bytes()); // count
    exif_ifd_bytes.extend_from_slice(&user_comment_data_offset.to_le_bytes()); // offset
    exif_ifd_bytes.extend_from_slice(&0u32.to_le_bytes()); // next IFD = 0
    exif_ifd_bytes.extend_from_slice(&user_comment_bytes);

    data_area.extend_from_slice(&exif_ifd_bytes);

    write_entry(0x010E, 2, image_desc.len() as u32, desc_offset, 0, &mut buffer);
    write_entry(0x010F, 2, make.len() as u32, make_offset, 1, &mut buffer);
    write_entry(0x0110, 2, model.len() as u32, model_offset, 2, &mut buffer);
    write_entry(0x0131, 2, software.len() as u32, soft_offset, 3, &mut buffer);
    write_entry(0x0132, 2, date_val.len() as u32, date_offset, 4, &mut buffer);
    write_entry(0x8769, 4, 1, exif_ifd_offset, 5, &mut buffer);

    buffer.extend_from_slice(&data_area);
    buffer
}

/// Builds an XMP packet containing ComfyUI workflow, prompt, and descriptions
pub fn build_comfy_xmp(workflow: Option<&str>, prompt: Option<&str>, date_str: Option<&str>) -> Vec<u8> {
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };

    let wf_escaped = esc(workflow.unwrap_or("{}"));
    let pr_escaped = esc(prompt.unwrap_or("{}"));
    let date_val = date_str.unwrap_or("2026-10-18T20:00:00");

    let xmp = format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:dc="http://purl.org/dc/elements/1.1/"
      xmlns:xmp="http://ns.adobe.com/xap/1.0/"
      xmlns:comfy="http://comfyui.org/workflow/">
      <dc:description>
        <rdf:Alt>
          <rdf:li xml:lang="x-default">Workflow: {}</rdf:li>
        </rdf:Alt>
      </dc:description>
      <xmp:CreateDate>{}</xmp:CreateDate>
      <comfy:prompt>{}</comfy:prompt>
      <comfy:workflow>{}</comfy:workflow>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        wf_escaped, date_val, pr_escaped, wf_escaped
    );

    xmp.into_bytes()
}

/// Builds the ComfyUI brob container box (Brotli-compressed JSON with "comf" FourCC payload)
pub fn build_comfy_brob_box(comfy: &ComfyMetadata) -> Option<Vec<u8>> {
    if comfy.prompt.is_none() && comfy.workflow.is_none() && comfy.parameters.is_none() {
        return None;
    }

    let mut map = serde_json::Map::new();
    if let Some(ref p) = comfy.prompt {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(p) {
            map.insert("prompt".to_string(), val);
        } else {
            map.insert("prompt".to_string(), serde_json::Value::String(p.clone()));
        }
    }
    if let Some(ref w) = comfy.workflow {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(w) {
            map.insert("workflow".to_string(), val);
        } else {
            map.insert("workflow".to_string(), serde_json::Value::String(w.clone()));
        }
    }
    if let Some(ref param) = comfy.parameters {
        map.insert("parameters".to_string(), serde_json::Value::String(param.clone()));
    }
    for (k, v) in &comfy.extra_text {
        if k != "prompt" && k != "workflow" && k != "parameters" {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(v) {
                map.insert(k.clone(), val);
            } else {
                map.insert(k.clone(), serde_json::Value::String(v.clone()));
            }
        }
    }

    let json_bytes = serde_json::to_vec(&map).ok()?;

    let mut compressed = Vec::new();
    {
        let mut writer = brotli::CompressorWriter::new(&mut compressed, 4096, 9, 22);
        std::io::Write::write_all(&mut writer, &json_bytes).ok()?;
    }

    let payload_tag = b"comf";
    let payload_len = 4 + compressed.len();
    let box_len = (8 + payload_len) as u32;

    let mut box_data = Vec::with_capacity(box_len as usize);
    box_data.extend_from_slice(&box_len.to_be_bytes());
    box_data.extend_from_slice(b"brob");
    box_data.extend_from_slice(payload_tag);
    box_data.extend_from_slice(&compressed);

    Some(box_data)
}

/// Injects the ComfyUI brob box into a JPEG XL container directly after the ftyp box
pub fn inject_comfy_brob_to_jxl(jxl_bytes: &[u8], comfy: &ComfyMetadata) -> Option<Vec<u8>> {
    let box_bytes = build_comfy_brob_box(comfy)?;

    if jxl_bytes.len() < 32 {
        return None;
    }

    // Verify JXL container signature: [0..4] = 12, [4..8] = b"JXL ", [8..12] = 0x0D0A870A
    if &jxl_bytes[4..8] != b"JXL " || jxl_bytes[8..12] != [0x0D, 0x0A, 0x87, 0x0A] {
        return None;
    }

    // Next box should be "ftyp"
    let ftyp_len = u32::from_be_bytes([jxl_bytes[12], jxl_bytes[13], jxl_bytes[14], jxl_bytes[15]]) as usize;
    if &jxl_bytes[16..20] != b"ftyp" || 12 + ftyp_len > jxl_bytes.len() {
        return None;
    }

    let insert_pos = 12 + ftyp_len;
    let mut result = Vec::with_capacity(jxl_bytes.len() + box_bytes.len());
    result.extend_from_slice(&jxl_bytes[..insert_pos]);
    result.extend_from_slice(&box_bytes);
    result.extend_from_slice(&jxl_bytes[insert_pos..]);

    Some(result)
}

fn extract_from_jxl(data: &[u8], out: &mut ExtractedMetadata) {
    if data.len() < 32 || &data[4..8] != b"JXL " {
        return;
    }
    let mut offset = 12;
    while offset + 8 <= data.len() {
        let size_32 = u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
        let box_type = &data[offset + 4..offset + 8];
        let (header_len, total_size) = if size_32 == 1 {
            if offset + 16 > data.len() {
                break;
            }
            let size_64 = u64::from_be_bytes([
                data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11],
                data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]
            ]) as usize;
            (16, size_64)
        } else if size_32 == 0 {
            (8, data.len() - offset)
        } else {
            (8, size_32)
        };

        if total_size < header_len || offset.checked_add(total_size).is_none_or(|end| end > data.len()) {
            break;
        }

        let payload = &data[offset + header_len..offset + total_size];

        if box_type == b"brob" && payload.len() > 4 {
            let orig_type = &payload[..4];
            let compressed = &payload[4..];
            let mut decompressed = Vec::new();
            let mut reader = brotli::Decompressor::new(compressed, 4096);
            if std::io::Read::read_to_end(&mut reader, &mut decompressed).is_ok() {
                if orig_type == b"comf" {
                    parse_comfy_json_bytes(&decompressed, out);
                } else if orig_type == b"Exif" {
                    out.raw_exif = Some(decompressed);
                    out.summary.has_exif = true;
                } else if orig_type == b"xml " {
                    out.raw_xmp = Some(decompressed);
                    out.summary.has_xmp = true;
                }
            }
        } else if box_type == b"comf" {
            parse_comfy_json_bytes(payload, out);
        } else if box_type == b"Exif" && payload.len() > 4 {
            let exif_data = &payload[4..]; // Skip 4-byte offset
            out.raw_exif = Some(exif_data.to_vec());
            out.summary.has_exif = true;
        } else if box_type == b"xml " {
            out.raw_xmp = Some(payload.to_vec());
            out.summary.has_xmp = true;
        }

        offset += total_size;
    }
}

fn parse_comfy_json_bytes(data: &[u8], out: &mut ExtractedMetadata) {
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(data)
        && let Some(obj) = val.as_object()
    {
        if let Some(p) = obj.get("prompt") {
                out.comfy.prompt = Some(if p.is_string() { p.as_str().unwrap().to_string() } else { p.to_string() });
                out.summary.has_comfy_workflow = true;
            }
            if let Some(w) = obj.get("workflow") {
                out.comfy.workflow = Some(if w.is_string() { w.as_str().unwrap().to_string() } else { w.to_string() });
                out.summary.has_comfy_workflow = true;
            }
            if let Some(param) = obj.get("parameters") {
                out.comfy.parameters = Some(if param.is_string() { param.as_str().unwrap().to_string() } else { param.to_string() });
                out.summary.has_comfy_workflow = true;
            }
    }
}

pub fn calc_crc32(data: &[u8]) -> u32 {
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

pub fn append_png_text_chunk(png: &[u8], keyword: &str, text: &str) -> Vec<u8> {
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

pub fn inject_comfy_metadata_into_png(png_path: &Path, comfy: &ComfyMetadata) -> Result<(), String> {
    if comfy.prompt.is_none() && comfy.workflow.is_none() && comfy.parameters.is_none() {
        return Ok(());
    }
    let mut data = std::fs::read(png_path).map_err(|e| format!("Failed to read PNG: {}", e))?;
    if let Some(ref p) = comfy.prompt {
        data = append_png_text_chunk(&data, "prompt", p);
    }
    if let Some(ref w) = comfy.workflow {
        data = append_png_text_chunk(&data, "workflow", w);
    }
    if let Some(ref param) = comfy.parameters {
        data = append_png_text_chunk(&data, "parameters", param);
    }
    for (k, v) in &comfy.extra_text {
        if k != "prompt" && k != "workflow" && k != "parameters" {
            data = append_png_text_chunk(&data, k, v);
        }
    }
    std::fs::write(png_path, data).map_err(|e| format!("Failed to write PNG: {}", e))?;
    Ok(())
}
