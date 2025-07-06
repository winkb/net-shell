use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 执行方式枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExecutionMethod {
    #[serde(rename = "ssh")]
    SSH,
    #[serde(rename = "websocket")]
    WebSocket,
}

/// SSH连接配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key_path: Option<String>,
    pub session_timeout_seconds: Option<u64>,
    pub timeout_seconds: Option<u64>,
}

/// WebSocket配置（预留，后续实现）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub url: String,
    pub timeout_seconds: Option<u64>,
}

/// 客户端配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    pub name: String,
    pub execution_method: ExecutionMethod,
    pub ssh_config: Option<SshConfig>,
    pub websocket_config: Option<WebSocketConfig>,
}

/// 变量提取规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractRule {
    pub name: String,
    pub patterns: Vec<String>, // 支持多个正则表达式，按顺序尝试直到匹配成功
    pub source: String, // "stdout", "stderr", "exit_code"
    #[serde(default = "default_cascade")]
    pub cascade: bool, // 是否启用级联模式：前一个正则的匹配结果作为下一个正则的输入，默认为true
}

/// 默认级联模式为true
fn default_cascade() -> bool {
    true
}

/// 步骤配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Step {
    pub name: String,
    pub script: String,
    #[serde(default)]
    pub servers: Vec<String>,
    pub timeout_seconds: Option<u64>,
    pub extract: Option<Vec<ExtractRule>>,
}

/// 流水线配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub name: String,
    pub steps: Vec<Step>,
}

/// 全局配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteExecutionConfig {
    pub variables: Option<HashMap<String, String>>,
    pub clients: HashMap<String, ClientConfig>,
    pub pipelines: Vec<Pipeline>,
    pub default_timeout: Option<u64>,
}

/// 实时输出类型
#[derive(Debug, Clone)]
pub enum OutputType {
    Stdout,
    Stderr,
    Log,
}

/// 实时输出事件
#[derive(Debug, Clone)]
pub struct OutputEvent {
    pub pipeline_name: String,
    pub server_name: String,
    pub step: Step, // 替换step_name为完整的Step对象，方便排错
    pub output_type: OutputType,
    pub content: String,
    pub timestamp: std::time::Instant,
    pub variables: HashMap<String, String>, // 添加当前变量上下文
}

/// 输出回调函数类型
pub type OutputCallback = std::sync::Arc<dyn Fn(OutputEvent) + Send + Sync>;

/// 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub script: String,
    pub exit_code: i32,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    pub step_name: String,
    pub server_name: String,
    pub execution_result: ExecutionResult,
    pub overall_success: bool,
    pub execution_time_ms: u64,
}

/// 流水线执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineExecutionResult {
    pub pipeline_name: String,
    pub step_results: Vec<StepExecutionResult>,
    pub overall_success: bool,
    pub total_execution_time_ms: u64,
} 