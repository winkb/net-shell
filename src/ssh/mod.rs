pub mod local;

use anyhow::{Context, Result};
use ssh2::Session;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::Duration;
use tokio::sync::mpsc as tokio_mpsc;
use tracing::info;

use crate::models::{ExecutionResult, SshConfig, OutputEvent, OutputType, OutputCallback};
use crate::Step;
use crate::vars::VariableManager;
use crate::ExtractRule;

/// SSH执行器
pub struct SshExecutor;

impl SshExecutor {
    /// 通过SSH执行脚本（支持实时输出）
    pub fn execute_script_with_realtime_output(
        server_name: &str,
        ssh_config: &SshConfig, 
        step: &Step,
        pipeline_name: &str,
        step_name: &str,
        output_callback: Option<OutputCallback>,
        mut variable_manager: VariableManager,
        extract_rules: Option<Vec<ExtractRule>>
    ) -> Result<ExecutionResult> {
        info!("Connecting to {}:{} as {}", ssh_config.host, ssh_config.port, ssh_config.username);

        // 只用step.script作为脚本路径，不做参数处理
        let script_path = step.script.as_str();
        // 读取本地脚本内容并替换变量
        let script_content = std::fs::read_to_string(script_path)
            .context(format!("Failed to read script file: {}", script_path))?;
        let script_content = variable_manager.replace_variables(&script_content);

        // 设置连接超时
        let timeout_seconds = step.timeout_seconds
            .or(ssh_config.timeout_seconds)
            .unwrap_or(3);
        let timeout_duration = Duration::from_secs(timeout_seconds);
        
        // 建立TCP连接（带严格超时）
        let tcp = connect_with_timeout(&format!("{}:{}", ssh_config.host, ssh_config.port), timeout_duration)
            .context("Failed to connect to SSH server")?;
        
        // 设置TCP连接超时
        tcp.set_read_timeout(Some(timeout_duration))
            .context("Failed to set read timeout")?;
        tcp.set_write_timeout(Some(timeout_duration))
            .context("Failed to set write timeout")?;
        tcp.set_nodelay(true)
            .context("Failed to set TCP nodelay")?;

        // 创建SSH会话
        let mut sess = Session::new()
            .context("Failed to create SSH session")?;
        
        sess.set_tcp_stream(tcp);
        
        // 设置SSH会话超时（使用步骤级别的超时，如果没有则使用默认值）
        let session_timeout_seconds = step.timeout_seconds.unwrap_or(30);
        let session_timeout_duration = Duration::from_secs(session_timeout_seconds);
        sess.set_timeout(session_timeout_duration.as_millis() as u32);
        
        // SSH握手（带超时）
        sess.handshake()
            .context("SSH handshake failed")?;

        info!("SSH handshake completed, starting authentication");

        // 认证（带超时）
        let auth_result = if let Some(ref password) = ssh_config.password {
            sess.userauth_password(&ssh_config.username, password)
                .context("SSH password authentication failed")
        } else if let Some(ref key_path) = ssh_config.private_key_path {
            sess.userauth_pubkey_file(&ssh_config.username, None, Path::new(key_path), None)
                .context("SSH key authentication failed")
        } else {
            Err(anyhow::anyhow!("No authentication method provided"))
        };

        auth_result?;
        info!("SSH authentication successful");

        // 打开远程shell
        let mut channel = sess.channel_session()
            .context("Failed to create SSH channel")?;
        channel.exec("sh")
            .context("Failed to exec remote shell")?;

        // 把脚本内容写入远程shell的stdin
        use std::io::Write;
        channel.write_all(script_content.as_bytes())
            .context("Failed to write script to remote shell")?;
        channel.send_eof()
            .context("Failed to send EOF to remote shell")?;

        // 创建通道用于实时输出
        let (tx, mut rx) = tokio_mpsc::channel::<OutputEvent>(100);
        let output_callback = output_callback.map(|cb| Arc::new(cb));

        // 在单独的线程中处理实时输出
        let server_name = server_name.to_string();
        let _step_name = step_name.to_string();
        let pipeline_name = pipeline_name.to_string();
        let output_callback_clone = output_callback.clone();
        
        let output_handle = std::thread::spawn(move || {
            while let Some(event) = rx.blocking_recv() {
                if let Some(callback) = &output_callback_clone {
                    callback(event);
                }
            }
        });

        // 读取stdout和stderr
        let mut stdout = String::new();
        let mut stderr = String::new();
        let start_time = std::time::Instant::now();

        // 实时读取stdout
        let stdout_stream = channel.stream(0);
        let mut stdout_reader = BufReader::new(stdout_stream);
        let mut line = String::new();
        
        while stdout_reader.read_line(&mut line)? > 0 {
            let content = line.clone();
            stdout.push_str(&content);
            
            // 发送实时输出事件
            let event = OutputEvent {
                pipeline_name: pipeline_name.clone(),
                server_name: server_name.clone(),
                step: step.clone(), // 传递完整的Step对象
                output_type: OutputType::Stdout,
                content: content.trim().to_string(),
                timestamp: std::time::Instant::now(),
                variables: variable_manager.get_variables().clone(),
            };
            
            if tx.blocking_send(event).is_err() {
                break;
            }
            
            line.clear();
        }

        // 实时读取stderr
        let stderr_stream = channel.stderr();
        let mut stderr_reader = BufReader::new(stderr_stream);
        line.clear();
        
        while stderr_reader.read_line(&mut line)? > 0 {
            let content = line.clone();
            stderr.push_str(&content);
            
            // 发送实时输出事件
            let event = OutputEvent {
                pipeline_name: pipeline_name.clone(),
                server_name: server_name.clone(),
                step: step.clone(), // 传递完整的Step对象
                output_type: OutputType::Stderr,
                content: content.trim().to_string(),
                timestamp: std::time::Instant::now(),
                variables: variable_manager.get_variables().clone(),
            };
            
            if tx.blocking_send(event).is_err() {
                break;
            }
            
            line.clear();
        }

        // 等待通道关闭
        drop(tx);
        if let Err(e) = output_handle.join() {
            eprintln!("Output handler thread error: {:?}", e);
        }

        channel.wait_close()
            .context("Failed to wait for channel close")?;

        let exit_code = channel.exit_status()
            .context("Failed to get exit status")?;

        let execution_time = start_time.elapsed().as_millis() as u64;
        info!("SSH command executed with exit code: {}", exit_code);

        // 创建执行结果
        let execution_result = ExecutionResult {
            success: exit_code == 0,
            stdout,
            stderr,
            script: step.script.to_string(),
            exit_code,
            execution_time_ms: execution_time,
            error_message: None,
        };

        // 提取变量
        if let Some(rules) = extract_rules {
            if let Err(e) = variable_manager.extract_variables(&rules, &execution_result) {
                info!("Failed to extract variables: {}", e);
            }
        }

        Ok(execution_result)
    }

}

/// 工具函数：带超时的TCP连接
fn connect_with_timeout(addr: &str, timeout: Duration) -> std::io::Result<TcpStream> {
    let (tx, rx) = mpsc::channel();
    let addr = addr.to_string();
    let error_message = format!("connect to {} timeout {} s", addr, timeout.as_secs());
    std::thread::spawn(move || {
        let res = TcpStream::connect(addr);
        let _ = tx.send(res);
    });
    rx.recv_timeout(timeout).unwrap_or_else(|_| Err(std::io::Error::new(std::io::ErrorKind::TimedOut, error_message)))
} 