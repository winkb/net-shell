# Net-Shell: 远程Shell脚本执行框架

Net-Shell 是一个用Rust编写的远程脚本执行和变量提取框架，支持通过SSH在远程服务器上执行shell脚本，并提供强大的变量提取和流水线编排功能。

## 功能特性

- ✅ SSH远程执行shell脚本
- 🔄 流水线编排和步骤管理
- 📝 YAML配置文件支持
- 🔐 支持密码和私钥认证
- ⏱️ 执行时间统计
- 📊 详细的执行结果（stdout、stderr、退出码）
- 🚀 异步执行支持
- 📝 完整的日志记录
- 🔍 正则表达式变量提取
- 🔗 级联变量提取支持
- 📋 实时输出回调

## 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
net-shell = "0.1.0"
```

## 配置

创建YAML配置文件（例如`config.yaml`）：

```yaml
variables:
  master_ip: "192.168.0.199"
  app_name: "myapp"
  version: "1.0.0"

clients:
  mac_server:
    name: "mac_server"
    execution_method: ssh
    ssh_config:
      host: "{{ master_ip }}"
      port: 22
      username: "li"
      private_key_path: "/Users/li/.ssh/id_rsa"
      timeout_seconds: 2 

pipelines:
  - name: "deploy_app"
    steps:
      - name: "get_system_info"
        script: "/path/to/get_system_info.sh"
        timeout_seconds: 5
        servers:
          - mac_server
        extract:
          - name: "os_version"
            patterns: ["OS Version: (.+)"]
            source: "stdout"
          - name: "hostname"
            patterns: ["Hostname: (.+)"]
            source: "stdout"
      
      - name: "deploy_application"
        script: "/path/to/deploy.sh"
        timeout_seconds: 10
        servers:
          - mac_server
        extract:
          - name: "deploy_path"
            patterns: ["Deployed to: (.+)"]
            source: "stdout"

default_timeout: 60
```

## 使用方法

### 基本用法

```rust
use net_shell::RemoteExecutor;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 从YAML文件创建执行器
    let executor = RemoteExecutor::from_yaml_file("config.yaml")?;

    // 执行所有流水线
    let results = executor.execute_all_pipelines().await?;
    
    // 打印执行结果摘要
    for result in &results {
        println!("Pipeline: {} ({})", result.pipeline_name, 
                 if result.overall_success { "Success" } else { "Failed" });
    }

    Ok(())
}
```

### 实时输出回调

```rust
use std::sync::Arc;
use net_shell::models::OutputEvent;

let output_callback = Arc::new(|event: OutputEvent| {
    match event.output_type {
        models::OutputType::Stdout => {
            println!("[STDOUT] {}: {}", event.server_name, event.content);
        }
        models::OutputType::Stderr => {
            eprintln!("[STDERR] {}: {}", event.server_name, event.content);
        }
        models::OutputType::Log => {
            println!("[LOG] {}: {}", event.server_name, event.content);
        }
    }
    
    // 显示当前变量状态
    if !event.variables.is_empty() {
        println!("[VARS] Current variables: {:?}", event.variables);
    }
});

let results = executor.execute_all_pipelines_with_realtime_output(
    Some(output_callback.clone()), 
    Some(output_callback)
).await?;
```

### 变量提取

Net-Shell 支持强大的变量提取功能：

```yaml
extract:
  - name: "os_version"
    patterns: ["OS Version: (.+)"]
    source: "stdout"
    cascade: true  # 默认启用级联模式
```

**变量提取约定：**
- 始终获取第一个捕获组（第一个括号）的内容
- 支持多个正则表达式作为备选方案
- 支持级联模式：前一个正则的输出作为下一个正则的输入

### 从字符串创建配置

```rust
let yaml_content = r#"
clients:
  server1:
    name: "server1"
    execution_method: ssh
    ssh_config:
      host: "192.168.1.100"
      port: 22
      username: "user"
      password: "password"
"#;

let executor = RemoteExecutor::from_yaml_str(yaml_content)?;
```

### 检查可用客户端

```rust
let available_clients = executor.get_available_clients();
println!("Available clients: {:?}", available_clients);

if executor.client_exists("server1") {
    println!("server1 is available");
}
```

## 配置说明

### SSH配置

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `host` | String | 是 | 服务器IP地址或域名 |
| `port` | u16 | 否 | SSH端口，默认22 |
| `username` | String | 是 | SSH用户名 |
| `password` | String | 否* | SSH密码（与private_key_path二选一） |
| `private_key_path` | String | 否* | 私钥文件路径（与password二选一） |
| `timeout_seconds` | u64 | 否 | 连接超时时间 |

* 必须提供password或private_key_path其中之一

### 变量提取配置

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `name` | String | 是 | 变量名称 |
| `patterns` | Vec<String> | 是 | 正则表达式模式列表 |
| `source` | String | 是 | 提取源（stdout/stderr/exit_code） |
| `cascade` | bool | 否 | 是否启用级联模式，默认true |

### 流水线配置

| 字段 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `name` | String | 是 | 流水线名称 |
| `steps` | Vec<Step> | 是 | 步骤列表 |
| `timeout_seconds` | u64 | 否 | 步骤超时时间 |

## 执行结果

`ExecutionResult`结构包含以下信息：

- `success`: 是否执行成功（exit_code == 0）
- `stdout`: 标准输出内容
- `stderr`: 标准错误输出内容
- `exit_code`: 脚本退出码
- `execution_time_ms`: 执行时间（毫秒）
- `error_message`: 错误信息（如果有）

## 错误处理

库使用`anyhow`进行错误处理，所有操作都返回`Result<T, anyhow::Error>`。常见错误包括：

- 配置文件解析错误
- SSH连接失败
- 认证失败
- 命令执行失败
- 网络超时
- 变量提取失败

## 示例

运行示例：

```bash
cargo run --bin main
```

## 测试

运行测试：

```bash
cargo test
```

## 特性

- **变量提取**: 支持正则表达式从脚本输出中提取变量
- **级联提取**: 支持多步骤变量提取，前一步的输出作为下一步的输入
- **流水线编排**: 支持复杂的多步骤流水线执行
- **实时输出**: 支持实时输出回调，便于监控和调试
- **变量替换**: 支持在配置中使用`{{ variable_name }}`进行变量替换
- **并发执行**: 同一步骤内的多个服务器并发执行

## 许可证

MIT OR Apache-2.0 