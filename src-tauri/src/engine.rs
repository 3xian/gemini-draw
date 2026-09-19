//! 与 `gemini-web-cli` 的对接层。
//!
//! Gemini Draw 不自己实现 Gemini Web 的私有协议，而是把
//! [Leechael/gemini-web-cli](https://github.com/Leechael/gemini-web-cli)
//! 当作引擎：所有网络与协议细节交给它，本应用只负责进程编排、产物落盘与 UI。

use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};

use crate::model::{AccountInfo, CliStatus, ModelInfo};

pub const REPO: &str = "Leechael/gemini-web-cli";

pub fn exe_name() -> &'static str {
    if cfg!(windows) {
        "gemini-web-cli.exe"
    } else {
        "gemini-web-cli"
    }
}

pub type ProgressFn = Arc<dyn Fn(String) + Send + Sync + 'static>;

// ---------------------------------------------------------------------------
// 定位 CLI
// ---------------------------------------------------------------------------

/// 按优先级查找 gemini-web-cli：手动指定 → 环境变量 → 内置目录 → 应用目录 → PATH
pub fn resolve_cli(root: &Path, explicit: Option<&str>) -> CliStatus {
    if let Some(p) = explicit {
        let p = p.trim();
        if !p.is_empty() {
            let p = PathBuf::from(p);
            if p.exists() {
                return status_of(Some(p), "手动指定");
            }
            return CliStatus {
                found: false,
                path: Some(p.to_string_lossy().into_owned()),
                version: None,
                source: "手动指定".into(),
                error: Some("配置的路径不存在".into()),
            };
        }
    }

    if let Ok(p) = std::env::var("GEMINI_WEB_CLI_PATH") {
        let p = PathBuf::from(p.trim());
        if p.exists() {
            return status_of(Some(p), "环境变量 GEMINI_WEB_CLI_PATH");
        }
    }

    let builtin = crate::paths::bin_dir(&root.to_path_buf()).join(exe_name());
    if builtin.exists() {
        return status_of(Some(builtin), "内置（已安装）");
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(exe_name());
            if sibling.exists() {
                return status_of(Some(sibling), "应用目录");
            }
            // macOS bundle: 可执行文件在 Contents/MacOS，资源在 Contents/Resources
            let resource = dir.join("../Resources").join(exe_name());
            if resource.exists() {
                return status_of(Some(resource), "应用资源目录");
            }
        }
    }

    if let Some(p) = find_in_path() {
        return status_of(Some(p), "PATH");
    }

    CliStatus {
        found: false,
        path: None,
        version: None,
        source: "未找到".into(),
        error: Some("未找到 gemini-web-cli，请在设置里安装或手动指定路径".into()),
    }
}

fn status_of(path: Option<PathBuf>, source: &str) -> CliStatus {
    let version = path.as_ref().and_then(|p| detect_version(p));
    CliStatus {
        found: path.is_some(),
        path: path.map(|p| p.to_string_lossy().into_owned()),
        version,
        source: source.into(),
        error: None,
    }
}

fn find_in_path() -> Option<PathBuf> {
    let sep = if cfg!(windows) { ';' } else { ':' };
    let path_var = std::env::var("PATH").ok()?;
    for dir in path_var.split(sep) {
        if dir.is_empty() {
            continue;
        }
        let candidate = Path::new(dir).join(exe_name());
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn detect_version(cli: &Path) -> Option<String> {
    let out = Command::new(cli).arg("--version").output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        return None;
    }
    // 形如 "gemini-web-cli version v1.2.3 (built ...)" 或 "v1.2.3"
    let cleaned = text
        .replace("gemini-web-cli version", "")
        .replace("gemini-web-cli", "");
    let first = cleaned
        .split_whitespace()
        .find(|t| !t.is_empty())
        .unwrap_or("")
        .to_string();
    if first.is_empty() {
        None
    } else {
        Some(first)
    }
}

// ---------------------------------------------------------------------------
// 执行 CLI
// ---------------------------------------------------------------------------

pub struct RunOpts<'a> {
    pub cookies: Option<&'a Path>,
    pub proxy: Option<&'a str>,
    pub account_index: Option<u32>,
    pub stdin: Option<&'a str>,
}

impl<'a> Default for RunOpts<'a> {
    fn default() -> Self {
        Self {
            cookies: None,
            proxy: None,
            account_index: None,
            stdin: None,
        }
    }
}

pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

fn global_args(opts: &RunOpts) -> Vec<String> {
    let mut args = Vec::new();
    if let Some(c) = opts.cookies {
        args.push("--cookies-json".into());
        args.push(c.to_string_lossy().into_owned());
    }
    if let Some(p) = opts.proxy {
        if !p.trim().is_empty() {
            args.push("--proxy".into());
            args.push(p.trim().into());
        }
    }
    if let Some(i) = opts.account_index {
        args.push("--account-index".into());
        args.push(i.to_string());
    }
    args
}

/// 执行 CLI 并在返回前持续把 stderr 行回调出去（用于上传/生成进度）。
pub fn run(
    cli: &Path,
    sub: &[&str],
    opts: &RunOpts,
    on_stderr: Option<ProgressFn>,
) -> Result<CmdOutput> {
    let mut args = global_args(opts);
    for s in sub {
        args.push((*s).to_string());
    }

    let mut cmd = Command::new(cli);
    cmd.args(&args)
        .stdin(if opts.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // 避免 Windows 上弹出控制台窗口
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }

    let mut child = cmd
        .spawn()
        .with_context(|| format!("启动 {} 失败", cli.display()))?;

    if let Some(text) = opts.stdin {
        if let Some(mut sin) = child.stdin.take() {
            let _ = std::io::Write::write_all(&mut sin, text.as_bytes());
        }
    }

    let stdout_handle = {
        let mut out = child.stdout.take().expect("stdout piped");
        std::thread::spawn(move || {
            let mut buf = String::new();
            let _ = out.read_to_string(&mut buf);
            buf
        })
    };

    let stderr_handle = {
        let err = child.stderr.take().expect("stderr piped");
        std::thread::spawn(move || {
            let reader = BufReader::new(err);
            let mut lines: Vec<String> = Vec::new();
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if let Some(cb) = &on_stderr {
                            cb(l.clone());
                        }
                        lines.push(l);
                    }
                    Err(_) => break,
                }
            }
            lines.join("\n")
        })
    };

    let status = child.wait()?;
    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();
    let code = status.code().unwrap_or(-1);
    Ok(CmdOutput {
        stdout,
        stderr,
        code,
    })
}

fn fail(out: &CmdOutput, what: &str) -> anyhow::Error {
    let detail = if !out.stderr.trim().is_empty() {
        out.stderr.trim().to_string()
    } else if !out.stdout.trim().is_empty() {
        out.stdout.trim().to_string()
    } else {
        format!("exit code {}", out.code)
    };
    anyhow!("{}失败：{}", what, detail)
}

// ---------------------------------------------------------------------------
// 具体命令
// ---------------------------------------------------------------------------

pub struct AskResult {
    pub text: String,
    pub images: Vec<String>,
    pub chat_id: Option<String>,
}

/// `ask --mode image [-f ref...] <prompt>`
pub fn ask_image(
    cli: &Path,
    prompt: &str,
    model: &str,
    refs: &[PathBuf],
    opts: &RunOpts,
    on_stderr: Option<ProgressFn>,
) -> Result<AskResult> {
    let mut sub: Vec<String> = vec![
        "ask".into(),
        "--no-stream".into(),
        "--mode".into(),
        "image".into(),
    ];
    if !model.trim().is_empty() {
        sub.push("--model".into());
        sub.push(model.trim().into());
    }
    for r in refs {
        sub.push("-f".into());
        sub.push(r.to_string_lossy().into_owned());
    }
    sub.push(prompt.to_string());

    let sub_refs: Vec<&str> = sub.iter().map(|s| s.as_str()).collect();
    let out = run(cli, &sub_refs, opts, on_stderr)?;
    if out.code != 0 {
        return Err(fail(&out, "生成图片"));
    }
    Ok(parse_ask_output(&out.stdout))
}

fn parse_indexed(line: &str) -> Option<String> {
    let t = line.trim();
    let close = t.find(')')?;
    let idx = &t[..close];
    if idx.is_empty() || !idx.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let rest = t[close + 1..].trim();
    let url = rest.split_whitespace().next()?.to_string();
    if url.starts_with("http") {
        Some(url)
    } else {
        None
    }
}

/// 解析 `ask` 的 stdout：
///
/// ```text
/// <正文>
/// ---
/// Generated images:
///   1) https://...
/// ---
/// Chat ID: c_xxx
/// ```
pub fn parse_ask_output(stdout: &str) -> AskResult {
    let lines: Vec<&str> = stdout.lines().collect();

    let mut text_end = lines.len();
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "---" {
            text_end = i;
            break;
        }
    }
    let text = lines[..text_end].join("\n").trim_end().to_string();

    let mut chat_id = None;
    let mut images: Vec<String> = Vec::new();
    let mut section: Option<&str> = None;

    for l in &lines[text_end..] {
        let t = l.trim_end();
        if let Some(rest) = t.trim_start().strip_prefix("Chat ID:") {
            chat_id = Some(rest.trim().to_string());
            section = None;
            continue;
        }
        match t.trim() {
            "---" => section = None,
            "Images:" => section = Some("web"),
            "Generated images:" => section = Some("generated"),
            "Generated videos:" | "Generated media:" => section = Some("other"),
            _ => {
                if matches!(section, Some("web") | Some("generated")) {
                    if let Some(u) = parse_indexed(t) {
                        if !images.contains(&u) {
                            images.push(u);
                        }
                    }
                }
            }
        }
    }

    AskResult {
        text,
        images,
        chat_id,
    }
}

/// `download <url> -o <path>`
pub fn download(cli: &Path, url: &str, target: &Path, opts: &RunOpts) -> Result<()> {
    let target_str = target.to_string_lossy().into_owned();
    let sub = ["download", url, "-o", &target_str];
    let out = run(cli, &sub, opts, None)?;
    if out.code != 0 {
        return Err(fail(&out, "下载图片"));
    }
    Ok(())
}

/// `import - -o <cookies.json>`（cookie 串走 stdin，避免超长命令行）
pub fn import_cookies(cli: &Path, cookie_text: &str, target: &Path) -> Result<()> {
    let target_str = target.to_string_lossy().into_owned();
    let sub = ["import", "-", "-o", &target_str];
    let opts = RunOpts {
        stdin: Some(cookie_text),
        ..Default::default()
    };
    let out = run(cli, &sub, &opts, None)?;
    if out.code != 0 {
        return Err(fail(&out, "导入 Cookie"));
    }
    Ok(())
}

/// `status` —— 登录诊断 + 账号可用模型
pub fn status(cli: &Path, opts: &RunOpts) -> Result<AccountInfo> {
    let out = run(cli, &["status"], opts, None)?;
    Ok(parse_status_output(&out.stdout, &out.stderr))
}

pub fn parse_status_output(stdout: &str, _stderr: &str) -> AccountInfo {
    let mut logged_in = false;
    let mut message = String::new();
    let mut user = None;
    let mut tier = None;
    let mut models: Vec<ModelInfo> = Vec::new();
    let mut in_models = false;

    for line in stdout.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Init:") {
            let v = rest.trim();
            if v.starts_with("OK") {
                logged_in = true;
                message = "已登录".into();
            } else {
                message = v.trim_start_matches("FAILED —").trim_start_matches("FAILED -").trim().into();
            }
            in_models = false;
            continue;
        }
        if let Some(rest) = t.strip_prefix("User:") {
            user = Some(rest.trim().to_string());
            in_models = false;
            continue;
        }
        if let Some(rest) = t.strip_prefix("Account status:") {
            tier = Some(rest.trim().to_string());
            in_models = false;
            continue;
        }
        if t.starts_with("Available models") {
            in_models = true;
            continue;
        }
        if in_models {
            if t.starts_with("=== ") || t.is_empty() {
                in_models = false;
                continue;
            }
            // 形如 `gemini-3-flash (Gemini 3 Flash) [advanced]`
            if let Some(open) = t.find('(') {
                let name = t[..open].trim().to_string();
                let rest = &t[open + 1..];
                let close = rest.find(')').unwrap_or(rest.len());
                let display = rest[..close].trim().to_string();
                let advanced = rest[close..].contains("advanced");
                if !name.is_empty() {
                    models.push(ModelInfo {
                        name,
                        display_name: display,
                        advanced_only: advanced,
                    });
                }
            }
        }
    }

    if message.is_empty() {
        message = if models.is_empty() {
            "无法获取账号信息".into()
        } else {
            "已登录".into()
        };
    }

    AccountInfo {
        logged_in,
        message,
        user,
        tier,
        models,
    }
}

// ---------------------------------------------------------------------------
// 自动安装 CLI
// ---------------------------------------------------------------------------

fn target_keywords() -> (&'static str, &'static str, bool) {
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "amd64"
    } else {
        "amd64"
    };
    if cfg!(target_os = "windows") {
        ("windows", arch, true)
    } else if cfg!(target_os = "macos") {
        ("darwin", arch, false)
    } else {
        ("linux", arch, false)
    }
}

/// 从 GitHub Releases 下载并安装 gemini-web-cli，返回安装后的可执行文件路径。
pub fn install_cli(root: &Path, on_progress: ProgressFn, proxy: Option<&str>) -> Result<PathBuf> {
    let (os_kw, arch_kw, is_zip) = target_keywords();
    let dest_dir = crate::paths::bin_dir(&root.to_path_buf());
    std::fs::create_dir_all(&dest_dir)?;

    on_progress("查询最新 release…".into());
    let api = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let client = build_client(proxy)?;
    let resp = client
        .get(&api)
        .header("Accept", "application/vnd.github+json")
        .send()
        .context("查询 GitHub Releases 失败")?;
    if !resp.status().is_success() {
        bail!("查询 GitHub Releases 失败：HTTP {}", resp.status());
    }
    let body: serde_json::Value = resp.json().context("解析 GitHub 响应失败")?;
    let assets = body
        .get("assets")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    let want_ext = if is_zip { ".zip" } else { ".tar.gz" };
    let mut pick: Option<(String, String)> = None;
    for a in &assets {
        let name = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
        if !name.ends_with(want_ext) || !name.contains(os_kw) {
            continue;
        }
        let exact = name == format!("gemini-web-cli-{}-{}{}", os_kw, arch_kw, want_ext);
        if !name.contains(arch_kw) && !exact {
            continue;
        }
        if let Some(url) = a.get("browser_download_url").and_then(|u| u.as_str()) {
            pick = Some((name.to_string(), url.to_string()));
            if exact {
                break;
            }
        }
    }
    let (asset_name, asset_url) = pick.ok_or_else(|| {
        anyhow!(
            "没有找到适用于 {}-{} 的发布包（期望后缀 {}）",
            os_kw,
            arch_kw,
            want_ext
        )
    })?;

    on_progress(format!("下载 {}…", asset_name));
    let bytes = client
        .get(&asset_url)
        .send()
        .context("下载失败")?
        .bytes()
        .context("读取下载内容失败")?
        .to_vec();

    on_progress("解压…".into());
    let dest = dest_dir.join(exe_name());
    extract_binary(&bytes, &dest, is_zip)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
    }

    on_progress("安装完成".into());
    Ok(dest)
}

fn build_client(proxy: Option<&str>) -> Result<reqwest::blocking::Client> {
    let mut builder = reqwest::blocking::Client::builder().user_agent("gemini-draw");
    if let Some(p) = proxy {
        if !p.trim().is_empty() {
            builder = builder.proxy(reqwest::Proxy::all(p.trim()).context("代理地址无效")?);
        }
    }
    Ok(builder.build()?)
}

fn extract_binary(bytes: &[u8], dest: &Path, is_zip: bool) -> Result<()> {
    let mut found = None;
    if is_zip {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).context("打开 zip 失败")?;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i).context("读取 zip 条目失败")?;
            let name = f
                .enclosed_name()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.ends_with(exe_name()) || name.ends_with("gemini-web-cli") {
                let mut buf = Vec::new();
                f.read_to_end(&mut buf)?;
                found = Some(buf);
                break;
            }
        }
    } else {
        let gz = flate2::read::GzDecoder::new(Cursor::new(bytes));
        let mut archive = tar::Archive::new(gz);
        for entry in archive.entries().context("读取 tar 失败")? {
            let mut entry = entry.context("读取 tar 条目失败")?;
            let name = entry
                .path()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            if name.ends_with(exe_name()) || name.ends_with("gemini-web-cli") {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                found = Some(buf);
                break;
            }
        }
    }

    let buf = found.context("发布包里没有找到 gemini-web-cli 可执行文件")?;
    std::fs::write(dest, buf).with_context(|| format!("写入 {} 失败", dest.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_generated_images_and_chat_id() {
        let stdout = "这是配图说明\n---\nGenerated images:\n  1) https://lh3.googleusercontent.com/aaa\n  2) https://lh3.googleusercontent.com/bbb\n---\nChat ID: c_1a2b3c\n";
        let r = parse_ask_output(stdout);
        assert_eq!(r.text, "这是配图说明");
        assert_eq!(
            r.images,
            vec![
                "https://lh3.googleusercontent.com/aaa".to_string(),
                "https://lh3.googleusercontent.com/bbb".to_string()
            ]
        );
        assert_eq!(r.chat_id.as_deref(), Some("c_1a2b3c"));
    }

    #[test]
    fn parses_web_images_with_titles() {
        let stdout = "正文\n---\nImages:\n  1) https://example.com/a.png  标题A\n---\nGenerated images:\n  1) https://lh3.googleusercontent.com/x\n";
        let r = parse_ask_output(stdout);
        assert_eq!(
            r.images,
            vec![
                "https://example.com/a.png".to_string(),
                "https://lh3.googleusercontent.com/x".to_string()
            ]
        );
    }

    #[test]
    fn parses_status_logged_in() {
        let stdout = "=== gemini-web-cli v1.2.3 (built x) ===\n\n=== Account Diagnostics ===\n  Model: unspecified\n  Init: OK (access token obtained)\n  User: Lee <lee@example.com>\n  Account status: available — 可用\n  Available models (2):\n    gemini-3-flash (Gemini 3 Flash)\n    gemini-3-pro (Gemini 3 Pro) [advanced]\n";
        let a = parse_status_output(stdout, "");
        assert!(a.logged_in);
        assert_eq!(a.user.as_deref(), Some("Lee <lee@example.com>"));
        assert_eq!(a.models.len(), 2);
        assert_eq!(a.models[0].name, "gemini-3-flash");
        assert_eq!(a.models[0].display_name, "Gemini 3 Flash");
        assert!(!a.models[0].advanced_only);
        assert!(a.models[1].advanced_only);
    }

    #[test]
    fn parses_status_failed() {
        let stdout = "=== gemini-web-cli v1.2.3 (built x) ===\n  Init: FAILED — session expired\n";
        let a = parse_status_output(stdout, "");
        assert!(!a.logged_in);
        assert_eq!(a.message, "session expired");
    }

    #[test]
    fn indexed_lines_only() {
        assert_eq!(
            parse_indexed("  3) https://x/y.png"),
            Some("https://x/y.png".to_string())
        );
        assert_eq!(parse_indexed("     Thumbnail: https://x/t.png"), None);
    }
}
