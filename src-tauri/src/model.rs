use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// 手动指定的 gemini-web-cli 路径；为空则自动查找
    pub cli_path: Option<String>,
    /// 模型名（gemini-web-cli 的 --model）
    pub model: String,
    /// 生成图片保存目录；为空则用应用默认目录
    pub output_dir: Option<String>,
    /// HTTP/SOCKS 代理
    pub proxy: Option<String>,
    /// 多账号序号（/u/N）
    pub account_index: Option<u32>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            cli_path: None,
            model: "unspecified".into(),
            output_dir: None,
            proxy: None,
            account_index: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefImage {
    pub id: String,
    pub name: String,
    /// 应用数据目录内的副本路径
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultImage {
    pub id: String,
    /// Google 侧原始 URL
    pub url: String,
    /// 已下载到本地的路径
    pub path: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub prompt: String,
    pub model: Option<String>,
    pub refs: Vec<RefImage>,
    pub results: Vec<ResultImage>,
    pub chat_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliStatus {
    pub found: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub source: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub logged_in: bool,
    pub message: String,
    pub user: Option<String>,
    pub tier: Option<String>,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub name: String,
    pub display_name: String,
    pub advanced_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSnapshot {
    pub app_version: String,
    pub data_dir: String,
    pub config: Config,
    pub cli: CliStatus,
    pub has_cookies: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub task_id: String,
    pub stage: String,
    pub message: String,
}
