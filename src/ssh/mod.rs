use anyhow::{Context, Result};
use ssh2::Session;
use std::io::{Read, BufRead, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::models::{ExecutionResult, SshConfig, OutputEvent, OutputType, OutputCallback};
use crate::Step;

/// SSH执行器
pub struct SshExecutor;

impl SshExecutor {
    /// 通过SSH执行脚本（支持实时输出）
    pub fn execute_script_with_realtime_output(
        server_name: &str,
        ssh_config: &SshConfig, 
        step: &Step,
        pipeline_name: &str,
        output_callback: Option<OutputCallback>
    ) -> Result<ExecutionResult> {
        info!("Connecting to {}:{} as {}", ssh_config.host, ssh_config.port, ssh_config.username);

        let script_path = Path::new(&step.script);
        // 读取脚本内容
        let script_content = std::fs::read_to_string(script_path)
            .context(format!("Failed to read script file: {}", step.script))?;

        // 建立TCP连接
        let tcp = TcpStream::connect(format!("{}:{}", ssh_config.host, ssh_config.port))
            .context("Failed to connect to SSH server")?;

        // 创建SSH会话
        let mut sess = Session::new()
            .context("Failed to create SSH session")?;
        
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .context("SSH handshake failed")?;

        // 认证
        if let Some(ref password) = ssh_config.password {
            sess.userauth_password(&ssh_config.username, password)
                .context("SSH password authentication failed")?;
        } else if let Some(ref key_path) = ssh_config.private_key_path {
            sess.userauth_pubkey_file(&ssh_config.username, None, Path::new(key_path), None)
                .context("SSH key authentication failed")?;
        } else {
            return Err(anyhow::anyhow!("No authentication method provided"));
        }

        info!("SSH authentication successful");

        // 执行命令
        let mut channel = sess.channel_session()
            .context("Failed to create SSH channel")?;

        channel.exec(&script_content)
            .context("Failed to execute command")?;

        // 创建通道用于实时输出
        let (tx, mut rx) = mpsc::channel::<OutputEvent>(100);
        let output_callback = output_callback.map(|cb| Arc::new(cb));

        // 在单独的线程中处理实时输出
        let server_name = server_name.to_string();
        let step_name = step.name.to_string();
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
                step_name: step_name.clone(),
                output_type: OutputType::Stdout,
                content: content.trim().to_string(),
                timestamp: std::time::Instant::now(),
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
                step_name: step_name.clone(),
                output_type: OutputType::Stderr,
                content: content.trim().to_string(),
                timestamp: std::time::Instant::now(),
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

        Ok(ExecutionResult {
            success: exit_code == 0,
            stdout,
            stderr,
            script: step.script.to_string(),
            exit_code,
            execution_time_ms: execution_time,
            error_message: None,
        })
    }

} 