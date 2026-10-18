<div align="center">

# OmniJXL

### High-Performance, Bidirectional JPEG XL (`.jxl`) Desktop Suite

**Convert raster and RAW images into next-generation JPEG XL — or unpack JXL back to PNG and JPG with zero data loss.**

[![Rust](https://img.shields.io/badge/Rust-1.99.1-orange?logo=rust)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-v2-blue?logo=tauri)](https://tauri.app/)
[![Svelte](https://img.shields.io/badge/Svelte-5-red?logo=svelte)](https://svelte.dev/)
[![libjxl](https://img.shields.io/badge/libjxl-0.12.0-teal)](https://github.com/libjxl/libjxl)
[![SIMD](https://img.shields.io/badge/SIMD-AVX2%20Active-brightgreen)](https://en.wikipedia.org/wiki/Advanced_Vector_Extensions)
[![License](https://img.shields.io/badge/License-MIT-purple.svg)](LICENSE)

</div>

---

## Overview

**OmniJXL** is a cross-platform desktop application engineered in **Rust (2024 edition, 1.99.1)** and **Tauri v2** with a sleek **Svelte 5** dark user interface. It delivers bidirectional image conversion between formats (PNG, JPEG, WebP, AVIF, TIFF, BMP, GIF, HDR, OpenEXR, Camera RAW) and the standardized **JPEG XL (`.jxl`, ISO/IEC 18181)** format.

Whether archiving terabytes of photography, preparing responsive web assets, or organizing AI art workflows, OmniJXL provides:

* **Dual-Tab Workflow**: One-click switching between **"Encode to .JXL"** and **"JXL to (PNG / JPG)"**.
* **ComfyUI Workflow Retention**: Embeds AI generation graphs (`prompt`, `workflow`, `parameters`) into compressed `.jxl` container boxes, and automatically reconstructs PNG `tEXt` chunks when unpacking back to `.png`.
* **Defensive Gatekeeper**: Strict dimension ceilings (`16,384 × 16,384`), pre-allocation memory caps (`256 MB`), and magic-byte header sniffing to neutralize decompression bombs and malformed files before allocation.
* **Batch Folder Ingestion**: Drag and drop entire folders to scan and process nested directories recursively.
* **Parallel Multithreading**: Work-stealing CPU saturation powered by **Rayon** and hardware **AVX2 SIMD** acceleration.
* **Live File Size Tracking**: Dynamic status bar showing initial file size in **Red** and converted file size in **Green** in `"MB"`.

---

## Why JPEG XL (`.jxl`)?

JPEG XL is the royalty-free image standard developed by the Joint Photographic Experts Group (ISO/IEC JTC 1/SC 29/WG 1). It was created from the ground up to replace JPEG, PNG, GIF, WebP, and AVIF by combining all their individual strengths into a single universal format.

### Comprehensive Format Comparison

| Feature / Metric | JPEG XL (`.jxl`) |  PNG | JPEG | WebP (`.webp`) | AVIF (`.avif`) |
|:---|:---:|:---:|:---:|:---:|:---:|
| **Lossless Compression** | **Superior** (35–50% smaller than PNG) | Baseline (Deflate) | ❌ None | Moderate | Good, but slow |
| **Lossy Compression** | **Best-in-class** (VarDCT Butteraugli) | ❌ None | Outdated (DCT blocky) | Decent, softens edges | High, but smudges textures |
| **Reversible JPEG Transcode** | **Yes (`-j 1`)** (20–35% size drop, bit-for-bit lossless) | ❌ Generation loss | ❌ Re-compression loss | ❌ Generation loss | ❌ Generation loss |
| **Alpha / Transparency** | **Full (8/16-bit, lossy & lossless)** | Lossless only | ❌ None | Yes | Yes |
| **16-bit Depth & HDR** | **Native (Integer & 32-bit Float)** | 16-bit integer | ❌ 8-bit only | ❌ 8-bit only | 10/12-bit |
| **Multi-Core Encoding Speed** | **Fast (SIMD AVX2/AVX-512)** | Fast | Very Fast | Fast | ⚠️ Extremely slow CPU encode |
| **Multi-Core Decoding Speed** | **Very Fast (30–80+ MP/s)** | Moderate | Fast | Fast | ⚠️ Slower decoding |
| **Progressive Decoding** | **Yes (Blur-to-sharp responsive)** | Interlaced (slow) | Progressive scan | ❌ None | ❌ None |
| **Metadata & AI Workflows** | **ISOBMFF (`brob`), EXIF, ICC, XMP** | Text chunks (`tEXt`) | EXIF APP1 only | Limited EXIF/XMP | Limited metadata boxes |
| **Generation Generation Loss** | **Zero with `-j 1` or Modular** | Zero | High degradation | High degradation | High degradation |

---

## Key Benefits of JPEG XL in OmniJXL

### 1. Lossless Reversible JPEG Transcoding (`-j 1`)
Traditional conversion from JPEG to WebP or AVIF requires decoding the image to raw pixels and re-compressing it, causing generational loss and new compression artifacts. 

JPEG XL can directly copy the original 8×8 DCT coefficients and Huffman tables without decompressing to pixels:
* Achieves **20% to 35% smaller file sizes** with **zero generation loss**.
* Decoders (including OmniJXL's tab) can reconstruct the exact, **bit-for-bit identical original JPEG**.

### 2. Modular Lossless Mode
For screenshots, line art, UI assets, and game textures:
* Outperforms PNG by **35% to 50%** in file size.
* Supports non-RGB color spaces, alpha channels, depth maps, and thermal channels without color distortion.

### 3. VarDCT Perceptual Visual Mode
Powered by Google's **Butteraugli** psycho-visual model:
* Adjust the psychovisual distance parameter from `0.1` (near-lossless mastering) to `1.0` (visually transparent) or `3.0` (high-density web delivery).
* Preserves high-frequency details, film grain, and intricate hair/fur textures that AVIF blurs or smudges.

### 4. End-to-End ComfyUI Workflow Preservation
AI artists often lose generation parameters and prompt when sharing or archiving images.
* **Forward (PNG ➔ JXL)**: OmniJXL scans `prompt`, `workflow`, and `parameters` from PNG text chunks and embeds them as compressed `brob` ISOBMFF boxes inside the `.jxl` container. Standard Adobe XMP packets are also synthesized.
* **Reverse (JXL ➔ PNG)**: Unpacking via the JXL to PNG / JPG tab extracts the `brob` box and injects valid PNG `tEXt` chunks. The unpacked PNG can be dragged directly into ComfyUI with all nodes, seeds, and models preserved.

### 5. 16-Bit Grayscale & Depth Map Retention
* Preserves 16-bit integer depth maps and floating-point normal buffers without banding, posterization, or clipping.

---

## Defensive Gatekeeper & Security Architecture

To protect systems against corrupted files, fuzzing exploits, and memory exhaustion attacks (decompression bombs), OmniJXL implements a multi-tier gatekeeper in `src-tauri/src/engine/limits.rs`:

```
[ Untrusted File Input ]
           │
           ▼
[ Magic-Byte Sniffer ] ─────────► Rejects file extension spoofing
           │
           ▼
[ Pre-Allocation Arithmetic ] ──► Calculates: width × height × bytes_per_pixel
           │                      Rejects if > 16,384 px or > 256 MB buffer
           ▼
[ Safe Reader (image::Limits) ] ─► Guards buffer allocations in pure Rust
           │
           ▼
[ Memory-Isolated Rayon Scheduler ] ──► Encodes / Decodes with zero crash propagation
```

* **Dimension Ceilings**: Strict maximum of `16,384 × 16,384` pixels (`MAX_IMAGE_WIDTH` / `MAX_IMAGE_HEIGHT`).
* **Memory Ceilings**: Hard single-buffer limit of `256 MB` (`268,435,456` bytes).
* **Decompression Bomb Defense**: Before allocating heap memory, OmniJXL executes overflow-checked multiplication (`checked_mul`). If an image attempts to allocate more than 256 MB, it triggers `ImageSecurityError::ExceedsMemoryLimit` without allocating a single byte.
* **Magic-Byte Sniffing**: Inspects binary header bytes (`FF D8 FF`, `89 50 4E 47...`, `00 00 00 0C 4A 58 4C 20`, `FF 0A`) to prevent malicious executables masquerading as images.
* **Path Traversal & CLI Injection Neutralization**: Canonicalizes all file paths to absolute paths (`std::path::absolute`) and sanitizes filenames against directory traversal (`..`) and argument injection (`-v.png`).

---

## Application Walkthrough

### Encode to .JXL

<div align="center">
  <p><i>Batch convert raw and raster images into next-generation JPEG XL with modular lossless or VarDCT modes.</i></p>
</div>

1. **Drop Images or Folders**:
   - Drag and drop individual image files onto the upper dashed dropzone.
   - Or drop a directory onto the **Batch Folder** dropzone to recursively scan and queue all nested images.
2. **Select Compression Mode**:
   - **Modular Lossless**: Exact bit-for-bit mathematical recreation (ideal for PNGs, graphics, and ComfyUI workflows).
   - **VarDCT Visual**: Psychovisually optimized lossy compression with Butteraugli distance tuning (`0.1` to `15.0`).
3. **Configure Settings**:
   - **Effort Slider**: 1 (Lightning) to 9 (Glacier/Max), default 7 (Tortoise).
   - **Progressive Decoding**: Enables responsive blur-to-sharp streaming.
   - **Copy Metadata**: Preserves EXIF, ICC profiles, and ComfyUI graphs.
   - **Reversible JPEG Transcode**: Enables lossless bitstream copy for `.jpg` files.
4. **Convert**:
   - Click **"Convert All to .JXL"** to launch parallel encoding across available CPU cores.

---

### JXL to (PNG / JPG)

<div align="center">
  <p><i>Turn .jxl images back into PNG or JPG for older software, image editors, or legacy web pipelines.</i></p>
</div>

1. **Switch Tabs**: Click **"JXL to (PNG / JPG)"** in the top header.
2. **Queue JXL Files**: Drop `.jxl` images or folders containing `.jxl` files.
3. **Select Target Format**:
   - **PNG Format**: Restores full alpha channels and automatically injects ComfyUI workflow graphs into PNG `tEXt` chunks.
   - **JPG Format**: High-quality JPEG conversion with adjustable quality slider (60–100%) and lossless JPEG transcode reconstruction.
4. **Unpack**:
   - Click **"Unpack All to .PNG"** (or `.JPG`). The resulting files are saved to your chosen destination folder.

---

### Live File Size Status Tracking
At the bottom of the window, OmniJXL displays real-time aggregate size metrics:
* **Initial Size**: Displayed in **Red text** (e.g. `24.50 MB`).
* **Converted Size**: Displayed in **Green text** (e.g. `8.12 MB`).
* **Savings Ratio**: Highlighted badge showing total percent saved (e.g. `-67%`).

---

## Supported Formats

| Format | Extension | Ingestion Pipeline | Target .JXL Mode | Legacy Unpack Target |
|---|---|---|---|---|
| **JPEG** | `.jpg`, `.jpeg` | Native / Reversible Transcode | Lossless DCT Transcode (`-j 1`) | `.jpg` / `.png` |
| **PNG / APNG** | `.png`, `.apng` | Native `image-rs` + Text chunks | Modular Lossless (`-m 1 -d 0`) | `.png` / `.jpg` |
| **JPEG XL** | `.jxl` | Pure-Rust sniff + `djxl` decoder | N/A (Input for Unpack) | `.png` / `.jpg` |
| **WebP** | `.webp` | Pure-Rust `image-rs` 16-bit | Modular or VarDCT | `.png` / `.jpg` |
| **AVIF** | `.avif` | Pure-Rust `image-rs` 16-bit | Modular or VarDCT | `.png` / `.jpg` |
| **TIFF** | `.tif`, `.tiff` | Pure-Rust `image-rs` 16-bit | Modular Lossless | `.png` / `.jpg` |
| **BMP** | `.bmp` | Pure-Rust `image-rs` 8/16-bit | Modular Lossless | `.png` / `.jpg` |
| **GIF** | `.gif` | Frame extraction | Modular Lossless | `.png` / `.jpg` |
| **OpenEXR** | `.exr` | High dynamic range buffer | Float32 Modular / VarDCT | `.png` / `.jpg` |
| **HDR** | `.hdr` | Radiance RGBE float buffer | Float32 Modular / VarDCT | `.png` / `.jpg` |
| **Camera RAW** | `.cr2`, `.nef`, `.arw`, `.dng` | RAW ingestion | High-Effort Modular | `.png` / `.jpg` |

---

## Build & Installation

### Prerequisites
* [Rust](https://rustup.rs/) (version 1.99.1 or higher)
* [Node.js](https://nodejs.org/) (version 20 or higher) and `npm`
* Visual Studio C++ Build Tools (on Windows)

### Development Mode
```bash
# Clone the repository
git clone https://github.com/your-username/OmniJXL.git
cd OmniJXL

# Install frontend dependencies
npm install

# Run Vite dev server and Tauri desktop application
npm run tauri dev
```

### Production Build
```bash
# Build frontend static bundle
npm run build

# Build native desktop executable (debug or release)
cargo build --manifest-path src-tauri/Cargo.toml --release
```
## License

OmniJXL is licensed under the [MIT License](LICENSE).
Built with love for high-fidelity imaging, digital preservation, and AI workflows.
