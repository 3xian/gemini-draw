use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine as _;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::engine::{self, RunOpts};
use crate::model::*;
use crate::paths;

pub struct AppState {
    pub root: PathBuf,
}

pub const EVT_PROGRESS: &str = "gemini-draw://progress";
pub const EVT_INSTALL: &str = "gemini-draw://cli-install";

fn root(app: &AppHandle) -> PathBuf {
    app.state::<AppState>().root.clone()
}

fn load_config(app: &AppHandle) -> Config {
    let root = root(app);
    paths::read_json::<Config>(&paths::config_path(&root))
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn require_cli(app: &AppHandle) -> Result<PathBuf> {
    let cfg = load_config(app);
    let status = engine::resolve_cli(&root(app), cfg.cli_path.as_deref());
    match status.path {
        Some(p) if status.found => Ok(PathBuf::from(p)),
        _ => Err(anyhow!(
            "{}",
            status
                .error
                .unwrap_or_else(|| "未找到 gemini-web-cli".to_string())
        )),
    }
}

fn run_opts<'a>(cfg: &'a Config, cookies: &'a Path) -> RunOpts<'a> {
    RunOpts {
        cookies: Some(cookies),
        proxy: cfg.proxy.as_deref(),
        account_index: cfg.account_index,
        stdin: None,
    }
}

fn has_cookies(root: &PathBuf) -> bool {
    let p = paths::cookies_path(root);
    if !p.exists() {
        return false;
    }
    let Ok(text) = std::fs::read_to_string(&p) else {
        return false;
    };
    let Ok(value): Result<serde_json::Value, _> = serde_json::from_str(&text) else {
        return false;
    };
    let map = value.get("cookies").unwrap_or(&value);
    map.get("__Secure-1PSID")
        .and_then(|v| v.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// 命令
// ---------------------------------------------------------------------------

#[tauri::command(rename_all = "snake_case")]
pub async fn app_snapshot(app: AppHandle) -> std::result::Result<AppSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<AppSnapshot> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        let cfg = load_config(&app);
        let cli = engine::resolve_cli(&root, cfg.cli_path.as_deref());
        Ok(AppSnapshot {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            data_dir: root.to_string_lossy().into_owned(),
            config: cfg,
            cli,
            has_cookies: has_cookies(&root),
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_config(app: AppHandle, config: Config) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        paths::write_json_atomic(&paths::config_path(&root), &config)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn install_cli(app: AppHandle) -> std::result::Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<String> {
        let root = root(&app);
        let cfg = load_config(&app);
        let cb: engine::ProgressFn = std::sync::Arc::new({
            let app = app.clone();
            move |msg: String| {
                let _ = app.emit(EVT_INSTALL, msg);
            }
        });
        let dest = engine::install_cli(&root, cb, cfg.proxy.as_deref())?;
        Ok(dest.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_cookies(
    app: AppHandle,
    cookie_text: String,
) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        let target = paths::cookies_path(&root);
        let text = cookie_text.trim();
        if text.is_empty() {
            bail!("Cookie 内容为空");
        }
        if !text.contains("__Secure-1PSID") && !text.contains("1PSID") {
            bail!("看起来不是有效的 Cookie 串：缺少 __Secure-1PSID");
        }
        let cli = require_cli(&app)?;
        engine::import_cookies(&cli, text, &target)?;
        if !target.exists() {
            bail!("导入后未生成 cookies.json");
        }
        // 用用户 Cookie 覆盖配置里的旧文件
        let _ = app.emit(EVT_PROGRESS, "Cookie 已导入");
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn check_account(app: AppHandle) -> std::result::Result<AccountInfo, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<AccountInfo> {
        let root = root(&app);
        if !has_cookies(&root) {
            bail!("还没有导入 Cookie");
        }
        let cli = require_cli(&app)?;
        let cfg = load_config(&app);
        let cookies = paths::cookies_path(&root);
        let opts = run_opts(&cfg, &cookies);
        engine::status(&cli, &opts)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn load_tasks(app: AppHandle) -> std::result::Result<Vec<Task>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<Task>> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        Ok(paths::read_json::<Vec<Task>>(&paths::tasks_path(&root))
            .ok()
            .flatten()
            .unwrap_or_default())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_tasks(app: AppHandle, tasks: Vec<Task>) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        paths::write_json_atomic(&paths::tasks_path(&root), &tasks)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

fn new_id() -> String {
    let a = rand::random::<u64>();
    let b = rand::random::<u64>();
    format!("{:x}{:x}", a, b)
}

fn safe_stem(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
            out.push(if c == ' ' { '_' } else { c });
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "image".to_string()
    } else if out.chars().count() > 40 {
        out.chars().take(40).collect()
    } else {
        out
    }
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_ref_file(
    app: AppHandle,
    task_id: String,
    src: String,
) -> std::result::Result<RefImage, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<RefImage> {
        let src = PathBuf::from(&src);
        if !src.is_file() {
            bail!("文件不存在：{}", src.display());
        }
        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into());
        let bytes = std::fs::read(&src).with_context(|| format!("读取 {} 失败", src.display()))?;
        store_ref(&app, &task_id, &name, &bytes)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn import_ref_base64(
    app: AppHandle,
    task_id: String,
    name: String,
    data: String,
) -> std::result::Result<RefImage, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<RefImage> {
        let raw = data.split(',').last().unwrap_or(&data);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.trim())
            .context("解码图片数据失败")?;
        store_ref(&app, &task_id, &name, &bytes)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

fn store_ref(app: &AppHandle, task_id: &str, name: &str, bytes: &[u8]) -> Result<RefImage> {
    let root = root(app);
    let dir = paths::refs_dir(&root, task_id);
    std::fs::create_dir_all(&dir)?;
    let ext = Path::new(name)
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_else(|| ".png".into());
    let id = new_id();
    let file = dir.join(format!("{}{}", id, ext));
    std::fs::write(&file, bytes)?;
    Ok(RefImage {
        id,
        name: name.to_string(),
        path: file.to_string_lossy().into_owned(),
        size: bytes.len() as u64,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn remove_ref(
    app: AppHandle,
    task_id: String,
    ref_id: String,
) -> std::result::Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<()> {
        let root = root(&app);
        let file = paths::refs_dir(&root, &task_id).join(format!("{}", ref_id));
        // 扩展名未知，按前缀匹配删除
        for entry in std::fs::read_dir(paths::refs_dir(&root, &task_id))?.flatten() {
            let n = entry.file_name().to_string_lossy().into_owned();
            if n.starts_with(&ref_id) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
        let _ = file;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

/// 执行生成：上传参考图 → 出图 → 下载到本地。
#[tauri::command(rename_all = "snake_case")]
pub async fn generate_task(
    app: AppHandle,
    task_id: String,
    title: String,
    prompt: String,
    model: String,
    refs: Vec<String>,
) -> std::result::Result<Vec<ResultImage>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Vec<ResultImage>> {
        let root = root(&app);
        paths::ensure_layout(&root)?;
        if !has_cookies(&root) {
            bail!("还没有导入 Cookie，请先在设置里导入");
        }
        let cli = require_cli(&app)?;
        let cfg = load_config(&app);
        let cookies = paths::cookies_path(&root);
        let opts = run_opts(&cfg, &cookies);

        let emit = |stage: &str, msg: &str| {
            let _ = app.emit(
                EVT_PROGRESS,
                ProgressEvent {
                    task_id: task_id.clone(),
                    stage: stage.into(),
                    message: msg.into(),
                },
            );
        };

        let files: Vec<PathBuf> = refs.iter().map(PathBuf::from).filter(|p| p.is_file()).collect();
        emit("prepare", "已就绪，正在连接 Gemini…");

        let cb: engine::ProgressFn = std::sync::Arc::new({
            let app = app.clone();
            let task_id = task_id.clone();
            move |line: String| {
                let t = line.trim();
                if t.is_empty() {
                    return;
                }
                let stage = if t.starts_with("Uploading") || t.starts_with("Uploaded") {
                    "upload"
                } else {
                    "generate"
                };
                let _ = app.emit(
                    EVT_PROGRESS,
                    ProgressEvent {
                        task_id: task_id.clone(),
                        stage: stage.into(),
                        message: t.to_string(),
                    },
                );
            }
        });

        if !files.is_empty() {
            emit("upload", &format!("上传 {} 张参考图…", files.len()));
        }
        emit("generate", "生成中，通常需要 10~60 秒…");

        let result = engine::ask_image(&cli, &prompt, &model, &files, &opts, Some(cb))?;

        if result.images.is_empty() {
            let hint = if result.text.trim().is_empty() {
                "Gemini 没有返回图片".to_string()
            } else {
                format!("Gemini 没有返回图片，仅返回文本：{}", truncate(&result.text, 200))
            };
            bail!("{}", hint);
        }

        let out_dir = paths::output_dir(&root, cfg.output_dir.as_deref());
        std::fs::create_dir_all(&out_dir)?;
        let stem = safe_stem(if title.trim().is_empty() {
            &prompt
        } else {
            &title
        });
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut results = Vec::new();
        for (i, url) in result.images.iter().enumerate() {
            emit("download", &format!("下载图片 {}/{}…", i + 1, result.images.len()));
            let target = out_dir.join(format!("{}-{}-{}.png", stem, now, i + 1));
            match engine::download(&cli, url, &target, &opts) {
                Ok(()) => results.push(ResultImage {
                    id: new_id(),
                    url: url.clone(),
                    path: Some(target.to_string_lossy().into_owned()),
                    created_at: now,
                }),
                Err(e) => {
                    emit("download", &format!("第 {} 张下载失败：{}", i + 1, e));
                    results.push(ResultImage {
                        id: new_id(),
                        url: url.clone(),
                        path: None,
                        created_at: now,
                    });
                }
            }
        }

        let tail = result
            .chat_id
            .map(|c| format!("（会话 {}）", c))
            .unwrap_or_default();
        emit("done", &format!("完成，共 {} 张{}", results.len(), tail));
        Ok(results)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let out: String = s.chars().take(max).collect();
        format!("{}…", out)
    }
}

/// 读取本地图片并缩放到指定边长以内，返回 base64 PNG，供前端直接展示。
#[tauri::command(rename_all = "snake_case")]
pub async fn read_image(
    path: String,
    max_side: Option<u32>,
) -> std::result::Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<String> {
        let img = image::open(&path).with_context(|| format!("无法打开图片 {}", path))?;
        let img = match max_side {
            Some(m) if m > 0 => {
                if img.width() > m || img.height() > m {
                    img.thumbnail(m, m)
                } else {
                    img
                }
            }
            _ => img,
        };
        let mut buf: Vec<u8> = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
            .context("编码图片失败")?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&buf))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn open_path(app: AppHandle, path: String) -> std::result::Result<(), String> {
    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn pick_directory(app: AppHandle) -> std::result::Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let picked = app.dialog().file().blocking_pick_folder();
        Ok(picked
            .and_then(|p| p.as_path().map(|x| x.to_string_lossy().into_owned())))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command(rename_all = "snake_case")]
pub async fn save_image_as(
    app: AppHandle,
    src: String,
    default_name: String,
) -> std::result::Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || -> Result<Option<String>> {
        let picked = app.dialog().file().set_file_name(&default_name).blocking_save_file();
        let Some(dest) = picked.and_then(|p| p.as_path().map(|x| x.to_path_buf())) else {
            return Ok(None);
        };
        std::fs::copy(&src, &dest).with_context(|| format!("保存到 {} 失败", dest.display()))?;
        Ok(Some(dest.to_string_lossy().into_owned()))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
