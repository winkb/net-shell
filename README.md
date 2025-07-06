# net-shell

A powerful script execution and variable extraction framework with SSH remote execution and local execution support, pipeline orchestration, and flexible variable extraction via regex.

## Features

- **Hybrid Execution**: Execute scripts both locally and remotely via SSH
- **Variable Extraction**: Extract variables from command outputs using regex patterns
- **Pipeline Orchestration**: Chain multiple steps with variable passing between them
- **Real-time Output**: Stream command outputs in real-time
- **Flexible Configuration**: YAML-based configuration with support for multiple servers
- **Error Handling**: Robust error handling and logging

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
net-shell = "0.2.0"
```

Or install the binary:

```bash
cargo install net-shell
```

## Quick Start

### Basic Configuration

Create a `config.yaml` file:

```yaml
steps:
  - name: "get_system_info"
    servers:
      - host: "192.168.1.100"
        username: "user"
        password: "password"
    script: |
      echo "System: $(uname -s)"
      echo "Hostname: $(hostname)"
      echo "Uptime: $(uptime)"
    variables:
      - name: "system_info"
        pattern: "System: (.+)"
      - name: "hostname"
        pattern: "Hostname: (.+)"
      - name: "uptime"
        pattern: "Uptime: (.+)"

  - name: "process_info"
    servers:
      - host: "192.168.1.100"
        username: "user"
        password: "password"
    script: |
      echo "Processing system: ${system_info}"
      echo "On host: ${hostname}"
      ps aux | head -5
```

### Local Execution

For local execution, simply omit the `servers` field or leave it empty:

```yaml
steps:
  - name: "local_check"
    script: |
      echo "Running locally on: $(hostname)"
      echo "Current user: $(whoami)"
      echo "Working directory: $(pwd)"
    variables:
      - name: "local_hostname"
        pattern: "Running locally on: (.+)"
```

### Mixed Local and Remote Execution

You can mix local and remote steps in the same pipeline:

```yaml
steps:
  - name: "local_prep"
    script: |
      echo "Preparing locally..."
      echo "Timestamp: $(date)"
    variables:
      - name: "timestamp"
        pattern: "Timestamp: (.+)"

  - name: "remote_execution"
    servers:
      - host: "192.168.1.100"
        username: "user"
        password: "password"
    script: |
      echo "Executing remotely with timestamp: ${timestamp}"
      echo "Remote host: $(hostname)"
```

## Usage

### Command Line

Run with default configuration (`config.yaml`):

```bash
cargo run --bin main
```

Or specify a custom configuration file:

```bash
cargo run --bin main config_custom.yaml
```

### Programmatic Usage

```rust
use net_shell::{Config, Executor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::from_file("config.yaml")?;
    
    // Create executor
    let executor = Executor::new(config);
    
    // Execute all steps
    executor.execute_all().await?;
    
    Ok(())
}
```

## Configuration Reference

### Step Configuration

Each step supports the following fields:

- `name`: Unique identifier for the step
- `servers`: List of SSH server configurations (optional for local execution)
- `script`: Shell script to execute
- `variables`: List of variable extraction patterns

### Server Configuration

```yaml
servers:
  - host: "192.168.1.100"
    username: "user"
    password: "password"
    # Optional: port (default: 22)
    port: 22
    # Optional: timeout in seconds (default: 30)
    timeout: 30
```

### Variable Extraction

Variables are extracted using regex patterns. Only the first capture group is used:

```yaml
variables:
  - name: "extracted_value"
    pattern: "Value: (.+)"
```

## Examples

### Complex Variable Extraction

```yaml
steps:
  - name: "extract_multiple"
    script: |
      echo "CPU: $(nproc) cores"
      echo "Memory: $(free -h | grep Mem | awk '{print $2}')"
      echo "Disk: $(df -h / | tail -1 | awk '{print $4}') available"
    variables:
      - name: "cpu_cores"
        pattern: "CPU: (.+) cores"
      - name: "memory_total"
        pattern: "Memory: (.+)"
      - name: "disk_available"
        pattern: "Disk: (.+) available"
```

### Pipeline with Variable Passing

```yaml
steps:
  - name: "step1"
    script: |
      echo "Step 1 completed"
      echo "Result: success"
    variables:
      - name: "step1_result"
        pattern: "Result: (.+)"

  - name: "step2"
    script: |
      echo "Step 1 result was: ${step1_result}"
      if [ "${step1_result}" = "success" ]; then
        echo "Step 2: processing..."
        echo "Status: completed"
      else
        echo "Step 2: skipped"
        echo "Status: skipped"
      fi
    variables:
      - name: "step2_status"
        pattern: "Status: (.+)"
```

## Error Handling

The framework provides comprehensive error handling:

- SSH connection failures
- Script execution errors
- Variable extraction failures
- Configuration validation errors

All errors are logged with appropriate context and stack traces.

## Implementation Example

Here's the complete implementation of the main program (`src/main.rs`) that demonstrates how to use the net-shell framework:

```rust
// 模块声明
pub mod config;
pub mod executor;
pub mod models;
pub mod ssh;
pub mod vars;

// 重新导出主要类型，方便外部使用
pub use executor::RemoteExecutor;
pub use models::*;

use std::{collections::HashMap, sync::Arc};
use tracing_subscriber;
use std::env;

// 主函数用于演示实时输出功能
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 解析命令行参数，支持指定配置文件路径
    let args: Vec<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        &args[1]
    } else {
        "config.yaml"
    };

    let mut variables = HashMap::new();
    variables.insert("new_master_ip".to_string(), "192.168.1.100".to_string());

    // 创建执行器
    let executor = RemoteExecutor::from_yaml_file(config_path, Some(variables))?;
    
    // 定义实时输出回调函数
    let output_callback = Arc::new(|event: models::OutputEvent| {
        let step = event.step.clone();
        
        match event.output_type {
            models::OutputType::Stdout => {
                println!("[STDOUT] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
            models::OutputType::Stderr => {
                eprintln!("[STDERR] {}@{}@{}: {}, script:[{}]", 
                         event.pipeline_name,
                         step.name,
                         event.server_name, 
                         event.content,
                         event.step.script
                        );
            }
            models::OutputType::Log => {
                println!("[LOG] {}@{}@{}: {}", 
                        event.pipeline_name,
                        step.name,
                        event.server_name, 
                        event.content);
            }
        }
        
        // 显示当前变量状态
        if !event.variables.is_empty() {
            println!("[VARS] Current variables: {:?}", event.variables);
        }
        
        // 显示步骤详细信息（如果有）
        println!("[STEP] Step details: name={}, script={}, servers={:?}, timeout={:?}, extract_rules={:?}", 
                step.name, step.script, step.servers, step.timeout_seconds, step.extract);
    });

    // 执行所有流水线
    let results = executor.execute_all_pipelines_with_realtime_output(Some(output_callback.clone()), Some(output_callback)).await?;
    
    // 打印执行结果摘要
    println!("\n=== 执行结果摘要 ===");
    for result in &results {
        println!("\n流水线: {} ({})", result.pipeline_name, 
                 if result.overall_success { "成功" } else { "失败" });
        println!("总执行时间: {}ms", result.total_execution_time_ms);
        println!("步骤结果:");
        
        for step_result in &result.step_results {
            let status = if step_result.execution_result.success { "✅" } else { "❌" };
            println!("  {} [{}:{}] {} - {}ms", 
                     status,
                     result.pipeline_name,
                     step_result.step_name,
                     step_result.server_name,
                     step_result.execution_result.execution_time_ms);
        }
    }
    
    // 统计总体结果
    let total_pipelines = results.len();
    let successful_pipelines = results.iter().filter(|r| r.overall_success).count();
    let total_steps = results.iter().map(|r| r.step_results.len()).sum::<usize>();
    let successful_steps = results.iter()
        .flat_map(|r| &r.step_results)
        .filter(|r| r.execution_result.success)
        .count();
    
    println!("\n=== 总体统计 ===");
    println!("流水线: {}/{} 成功", successful_pipelines, total_pipelines);
    println!("步骤: {}/{} 成功", successful_steps, total_steps);
    
    Ok(())
}
```

### Key Features Demonstrated:

1. **Command Line Arguments**: Supports specifying custom configuration file paths
2. **Real-time Output**: Implements comprehensive real-time output handling with different output types
3. **Variable Tracking**: Shows current variable state during execution
4. **Detailed Logging**: Provides step-by-step execution details
5. **Result Summary**: Generates comprehensive execution reports
6. **Statistics**: Calculates success rates for pipelines and steps

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.
