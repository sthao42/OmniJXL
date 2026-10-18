use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub threads: usize,
    pub simd: String,
    pub libjxl_version: String,
    pub cjxl_path: Option<String>,
    pub djxl_path: Option<String>,
    pub is_ready: bool,
}

pub fn find_libjxl_tool(tool_name: &str) -> Option<PathBuf> {
    let name_exe = if cfg!(windows) && !tool_name.ends_with(".exe") {
        format!("{}.exe", tool_name)
    } else {
        tool_name.to_string()
    };

    // 1. Direct which search in PATH
    if let Ok(path) = which::which(tool_name)
        && path.is_file()
    {
        return Some(path);
    }
    if let Ok(path) = which::which(&name_exe)
        && path.is_file()
    {
        return Some(path);
    }

    // 2. WinGet package default directory on Windows
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let winget_path = PathBuf::from(&local_app_data)
            .join("Microsoft")
            .join("WinGet")
            .join("Packages")
            .join("libjxl.libjxl_Microsoft.Winget.Source_8wekyb3d8bbwe")
            .join("x64-windows-static")
            .join("bin")
            .join(&name_exe);
        if winget_path.is_file() {
            return Some(winget_path);
        }

        let winget_link = PathBuf::from(&local_app_data)
            .join("Microsoft")
            .join("WinGet")
            .join("Links")
            .join(&name_exe);
        if winget_link.is_file() {
            return Some(winget_link);
        }
    }

    // 3. Search relative to current executable or app directory
    if let Ok(current_exe) = std::env::current_exe()
        && let Some(exe_dir) = current_exe.parent()
    {
        let local_bin = exe_dir.join(&name_exe);
        if local_bin.is_file() {
            return Some(local_bin);
        }
        let parent_bin = exe_dir.join("bin").join(&name_exe);
        if parent_bin.is_file() {
            return Some(parent_bin);
        }
    }

    // 4. Current working directory ./bin/
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_bin = cwd.join("bin").join(&name_exe);
        if cwd_bin.is_file() {
            return Some(cwd_bin);
        }
    }

    None
}

pub fn find_cjxl_executable() -> Option<PathBuf> {
    find_libjxl_tool("cjxl")
}

pub fn find_djxl_executable() -> Option<PathBuf> {
    find_libjxl_tool("djxl")
}

pub fn find_jxlinfo_executable() -> Option<PathBuf> {
    find_libjxl_tool("jxlinfo")
}

pub fn detect_simd() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx512f") {
            "SIMD AVX-512 active".to_string()
        } else if is_x86_feature_detected!("avx2") {
            "SIMD AVX2 active".to_string()
        } else if is_x86_feature_detected!("sse4.2") {
            "SIMD SSE4.2 active".to_string()
        } else {
            "SIMD Baseline x86_64".to_string()
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        "NEON SIMD active".to_string()
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        "Standard SIMD".to_string()
    }
}

pub fn get_system_info() -> SystemInfo {
    let threads = num_cpus::get();
    let simd = detect_simd();
    let cjxl_path_buf = find_cjxl_executable();
    let cjxl_path = cjxl_path_buf
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    let (libjxl_version, is_ready) = if let Some(ref path) = cjxl_path_buf {
        let mut check_cmd = Command::new(path);
        check_cmd.arg("-V");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            check_cmd.creation_flags(0x08000000);
        }
        match check_cmd.output() {
            Ok(output) => {
                let out_str = String::from_utf8_lossy(&output.stdout);
                let err_str = String::from_utf8_lossy(&output.stderr);
                let combined = format!("{} {}", out_str, err_str);
                let version_line = combined
                    .lines()
                    .find(|l| l.contains("cjxl") || l.contains("v0.") || l.contains("JPEG XL"))
                    .unwrap_or("libjxl 0.12.0")
                    .trim()
                    .to_string();
                (version_line, true)
            }
            Err(_) => ("libjxl detected (call failed)".to_string(), false),
        }
    } else {
        ("libjxl not found on path".to_string(), false)
    };

    let djxl_path_buf = find_djxl_executable();
    let djxl_path = djxl_path_buf
        .as_ref()
        .map(|p| p.to_string_lossy().to_string());

    SystemInfo {
        threads,
        simd,
        libjxl_version,
        cjxl_path,
        djxl_path,
        is_ready,
    }
}
