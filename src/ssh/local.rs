use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tracing::{error, info};

use crate::models::{ExecutionResult, OutputCallback, OutputEvent, OutputType, Step};

/// 本地脚本执行器
pub struct LocalExecutor;

impl LocalExecutor {
    /// 在本地执行shell脚本（支持实时输出）
    pub async fn execute_script_with_realtime_output(
        step: &Step,
        pipeline_name: &str,
        _step_name: &str,
        output_callback: Option<OutputCallback>,
        variables: std::collections::HashMap<String, String>,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        let pipeline_name = pipeline_name.to_string();
        
        // 检查脚本文件是否存在
        let script_path = std::path::Path::new(&step.script);
        if !script_path.exists() {
            return Err(anyhow::anyhow!("Script '{}' not found", step.script));
        }

        info!("Executing local script: {}", step.script);

        // 发送开始执行的日志
        if let Some(callback) = &output_callback {
            let event = OutputEvent {
                pipeline_name: pipeline_name.clone(),
                server_name: "localhost".to_string(),
                step: step.clone(),
                output_type: OutputType::Log,
                content: format!("开始执行本地脚本: {}", step.script),
                timestamp: Instant::now(),
                variables: variables.clone(),
            };
            callback(event);
        }

        // 设置超时
        let timeout_seconds = step.timeout_seconds.unwrap_or(60);
        
        // 创建异步命令
        let mut command = TokioCommand::new("bash");
        command.arg(&step.script);
        command.current_dir(std::env::current_dir()?);
        
        // 设置环境变量
        for (key, value) in &variables {
            command.env(key, value);
        }

        // 设置标准输出和错误输出
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // 执行命令
        let mut child = command.spawn()
            .context("Failed to spawn local script process")?;

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");

        // 克隆必要的数据用于异步任务
        let step_clone = step.clone();
        let pipeline_name1 = pipeline_name.clone();
        let pipeline_name2 = pipeline_name.clone();
        let variables_clone = variables.clone();
        let variables_clone2 = variables.clone();
        let output_callback_clone = output_callback.clone();
        let output_callback_clone2 = output_callback.clone();

        // 创建输出读取任务
        let stdout_task = tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut content = String::new();
            
            while let Ok(Some(line)) = lines.next_line().await {
                content.push_str(&line);
                content.push('\n');
                
                // 发送实时输出
                if let Some(callback) = &output_callback_clone {
                    let event = OutputEvent {
                        pipeline_name: pipeline_name1.to_string(),
                        server_name: "localhost".to_string(),
                        step: step_clone.clone(),
                        output_type: OutputType::Stdout,
                        content: line,
                        timestamp: Instant::now(),
                        variables: variables_clone.clone(),
                    };
                    callback(event);
                }
            }
            content
        });

        let step_clone2 = step.clone();
        let stderr_task = tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            let mut content = String::new();
            
            while let Ok(Some(line)) = lines.next_line().await {
                content.push_str(&line);
                content.push('\n');
                
                // 发送实时输出
                if let Some(callback) = &output_callback_clone2 {
                    let event = OutputEvent {
                        pipeline_name: pipeline_name2.to_string(),
                        server_name: "localhost".to_string(),
                        step: step_clone2.clone(),
                        output_type: OutputType::Stderr,
                        content: line,
                        timestamp: Instant::now(),
                        variables: variables_clone2.clone(),
                    };
                    callback(event);
                }
            }
            content
        });

        // 等待命令完成（带超时）
        let status = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds),
            child.wait()
        ).await;

        let exit_code = match status {
            Ok(Ok(exit_status)) => {
                exit_status.code().unwrap_or(-1)
            }
            Ok(Err(e)) => {
                error!("Local script execution failed: {}", e);
                return Err(anyhow::anyhow!("Local script execution failed: {}", e));
            }
            Err(_) => {
                // 超时，强制终止进程
                let _ = child.kill().await;
                return Err(anyhow::anyhow!("Local script execution timed out after {} seconds", timeout_seconds));
            }
        };

        // 等待输出读取完成
        let (stdout_result, stderr_result) = tokio::join!(stdout_task, stderr_task);
        
        let stdout_content = stdout_result.unwrap_or_default();
        let stderr_content = stderr_result.unwrap_or_default();

        let execution_time = start_time.elapsed().as_millis() as u64;
        let success = exit_code == 0;

        info!("Local script '{}' completed with exit code: {}", step.script, exit_code);

        // 发送完成日志
        if let Some(callback) = &output_callback {
            let status = if success { "成功" } else { "失败" };
            let event = OutputEvent {
                pipeline_name: pipeline_name.to_string(),
                server_name: "localhost".to_string(),
                step: step.clone(),
                output_type: OutputType::Log,
                content: format!("本地脚本执行完成: {} ({}) - 耗时: {}ms", step.script, status, execution_time),
                timestamp: Instant::now(),
                variables: variables.clone(),
            };
            callback(event);
        }

        Ok(ExecutionResult {
            success,
            stdout: stdout_content,
            stderr: stderr_content,
            script: step.script.clone(),
            exit_code,
            execution_time_ms: execution_time,
            error_message: if success { None } else { Some(format!("Script exited with code {}", exit_code)) },
        })
    }

    /// 在本地执行shell脚本（同步版本，用于兼容性）
    pub fn execute_script(step: &Step) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        
        // 检查脚本文件是否存在
        let script_path = std::path::Path::new(&step.script);
        if !script_path.exists() {
            return Err(anyhow::anyhow!("Script '{}' not found", step.script));
        }

        info!("Executing local script: {}", step.script);

        // 设置超时（注意：同步版本无法真正实现超时，这里只是记录）
        let _timeout_seconds = step.timeout_seconds.unwrap_or(60);
        
        // 创建命令
        let output = Command::new("bash")
            .arg(&step.script)
            .current_dir(std::env::current_dir()?)
            .output()
            .context("Failed to execute local script")?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let exit_code = output.status.code().unwrap_or(-1);
        let success = exit_code == 0;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        info!("Local script '{}' completed with exit code: {}", step.script, exit_code);

        Ok(ExecutionResult {
            success,
            stdout,
            stderr,
            script: step.script.clone(),
            exit_code,
            execution_time_ms: execution_time,
            error_message: if success { None } else { Some(format!("Script exited with code {}", exit_code)) },
        })
    }
} 