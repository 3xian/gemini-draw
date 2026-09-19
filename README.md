# Gemini Draw

用 Gemini 画图的跨平台桌面应用（Windows / macOS）。每个任务可以放若干张**参考图**（支持拖拽调整顺序）+ 一段 Prompt，交给 Gemini 出图，结果自动下载到本地并可以预览/另存。

## 它怎么工作

Gemini Draw **不自己实现** Gemini Web 的私有协议，而是把
[`Leechael/gemini-web-cli`](https://github.com/Leechael/gemini-web-cli) 当作引擎：

| 能力 | 使用的 CLI 命令 |
| --- | --- |
| 登录态 | `import`（导入浏览器 Cookie） |
| 出图（含参考图） | `ask --mode image -f 参考图1 -f 参考图2 "<prompt>"` |
| 下载原图 | `download <url> -o <本地路径>` |
| 账号/模型诊断 | `status` |

好处：协议细节（batchexecute / StreamGenerate / 可续传上传 / Cookie 刷新）全部由上游维护，
本仓库只负责进程编排、产物落盘和界面。Cookie 也是浏览器登录态，**不需要 API Key**。

技术栈：

- **Tauri 2 + Rust**：进程编排、文件与任务持久化、缩略图
- **React 18 + TypeScript + Vite**：界面

## 快速开始

```bash
npm install
npm run tauri dev
```

首次启动需要两件事：

1. **安装引擎**：设置 → 「自动安装 / 更新」，会自动从 GitHub Releases 下载匹配当前平台的
   `gemini-web-cli` 到应用数据目录。也可以手动指定已安装的路径。
2. **导入 Cookie**：浏览器打开 <https://gemini.google.com> 并登录 → 开发者工具 → Network →
   任选一个 `gemini.google.com` 请求 → 复制请求头里的整条 `Cookie` → 粘贴到设置里导入。
   （至少要有 `__Secure-1PSID`，建议同时带上 `__Secure-1PSIDTS`。）

然后「检查登录状态」确认可用即可开始出图。

## 使用

- 左侧管理任务；每个任务包含标题、Prompt、参考图列表和生成结果。
- 参考图：点击选择 / 从系统拖拽文件进窗口 / 直接拖到参考图区域。**拖动卡片可调整顺序**，
  顺序会原样传给 Gemini，会影响出图结果。
- 「生成图片」会实时显示上传与生成进度，完成后自动下载原图到本地输出目录。
- 结果支持点击放大、另存为、在文件夹中显示。

## 构建

```bash
# Windows（在当前机器上）
npm run tauri build

# macOS
npm run tauri build -- --target aarch64-apple-darwin   # Apple Silicon
npm run tauri build -- --target x86_64-apple-darwin    # Intel
```

macOS 交叉编译需要 macOS 机器（或在 macOS 上直接构建），因为 Tauri 需要链接 macOS SDK。

图标由 `scripts/make_icons.py` 生成（纯标准库，无需 Pillow）：

```bash
python scripts/make_icons.py
```

## 目录结构

```
src/                前端（React + TS）
src-tauri/src/
  engine.rs         调用 gemini-web-cli：定位、执行、解析输出、自动安装
  commands.rs       Tauri 命令
  paths.rs          应用数据目录布局
  store/model.rs    配置与任务模型
scripts/            图标生成等辅助脚本
```

应用数据目录：

- Windows：`%APPDATA%\gemini-draw`
- macOS：`~/Library/Application Support/gemini-draw`

其中 `cookies.json`（登录态）、`tasks.json`（任务）、`refs/`（参考图副本）、
`output/`（生成结果）、`bin/`（CLI 引擎）。

## 说明

- 本项目使用 Google Gemini 的网页版接口，请遵守 Google 的服务条款，并自行承担使用风险。
- 生成的图片版权与合规问题由使用者负责。
