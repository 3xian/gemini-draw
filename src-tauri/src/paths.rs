use std::path::PathBuf;

use anyhow::{Context, Result};

/// 应用数据根目录：
/// - Windows: %APPDATA%/gemini-draw
/// - macOS:   ~/Library/Application Support/gemini-draw
pub fn data_dir() -> Result<PathBuf> {
    let base = dirs::data_dir().context("无法确定应用数据目录")?;
    Ok(base.join("gemini-draw"))
}

pub fn ensure_layout(root: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(root)?;
    std::fs::create_dir_all(root.join("refs"))?;
    std::fs::create_dir_all(root.join("output"))?;
    std::fs::create_dir_all(root.join("bin"))?;
    Ok(())
}

pub fn config_path(root: &PathBuf) -> PathBuf {
    root.join("config.json")
}

pub fn tasks_path(root: &PathBuf) -> PathBuf {
    root.join("tasks.json")
}

pub fn cookies_path(root: &PathBuf) -> PathBuf {
    root.join("cookies.json")
}

pub fn bin_dir(root: &PathBuf) -> PathBuf {
    root.join("bin")
}

pub fn refs_dir(root: &PathBuf, task_id: &str) -> PathBuf {
    root.join("refs").join(task_id)
}

pub fn output_dir(root: &PathBuf, configured: Option<&str>) -> PathBuf {
    match configured {
        Some(p) if !p.trim().is_empty() => PathBuf::from(p),
        _ => root.join("output"),
    }
}

/// 原子写文件，避免写一半崩溃导致配置损坏。
pub fn write_json_atomic<T: serde::Serialize>(path: &PathBuf, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(value)?;
    std::fs::write(&tmp, data)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let data = std::fs::read_to_string(path)?;
    if data.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&data)?))
}
