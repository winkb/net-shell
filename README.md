# 远程Shell脚本执行库

这是一个用Rust编写的远程shell脚本执行库，支持通过SSH和WebSocket（计划中）方式在远程服务器上执行shell脚本。

## 功能特性

- ✅ SSH远程执行shell脚本
- 🔄 WebSocket远程执行（计划中）
- 📝 YAML配置文件支持
- 🔐 支持密码和私钥认证
- ⏱️ 执行时间统计
- 📊 详细的执行结果（stdout、stderr、退出码）
- 🚀 异步执行支持
- 📝 完整的日志记录

## 安装

在`Cargo.toml`中添加依赖：

```toml
[dependencies]
ai-demo = { path = "." }
```

## 配置

创建YAML配置文件（例如`config.yaml`）：

```yaml
clients:
  server1:
    name: "server1"
    execution_method: ssh
    ssh_config:
      host: "192.168.1.100"
      port: 22
      username: "user"
      password: "password"
      timeout_seconds: 30
  server2:
    name: "server2"
    execution_method: ssh
    ssh_config:
      host: "192.168.1.101"
      port: 22
      username: "admin"
      private_key_path: "/path/to/private/key"
      timeout_seconds: 30
default_timeout: 60
```

## 使用方法

### 基本用法

```rust
use ai_demo::RemoteExecutor;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 从YAML文件创建执行器
    let executor = RemoteExecutor::from_yaml_file("config.yaml")?;

    // 执行shell脚本
    let script = "echo 'Hello from remote server' && date";
    let result = executor.execute_script("server1", script).await?;

    println!("Success: {}", result.success);
    println!("Exit code: {}", result.exit_code);
    println!("Stdout: {}", result.stdout);
    println!("Execution time: {}ms", result.execution_time_ms);

    Ok(())
}
```

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

### 执行方式

- `ssh`: 通过SSH连接执行（已实现）
- `websocket`: 通过WebSocket发送消息执行（计划中）

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

## 示例

运行示例：

```bash
cargo run --example basic_usage
```

## 测试

运行测试：

```bash
cargo test
```

## 计划功能

- [ ] WebSocket执行支持
- [ ] 批量执行
- [ ] 执行超时控制
- [ ] 重试机制
- [ ] 更详细的错误信息
- [ ] 配置文件验证
- [ ] 支持环境变量传递

## 许可证

MIT License 